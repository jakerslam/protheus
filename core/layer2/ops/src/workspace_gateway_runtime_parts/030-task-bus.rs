use async_nats::jetstream;
use async_nats::jetstream::consumer::pull;
use async_nats::jetstream::stream::Config as StreamConfig;
use futures::StreamExt;
use tokio::runtime::Runtime;

// TEMPORARY SCAFFOLDING — NATS JetStream. To be replaced with native InfRing task ions built from baryons later.
trait TaskBus {
    fn mode(&self) -> &'static str;
    fn enqueue(&self, payload: &TaskPayload) -> Result<(), String>;
    fn dequeue(&self, max_messages: usize, wait_ms: u64) -> Result<Vec<TaskPayload>, String>;
    fn publish_cancel(&self, task_id: &str) -> Result<(), String>;
    fn pull_cancelled(&self, max_messages: usize, wait_ms: u64) -> Result<Vec<String>, String>;
}

struct LocalFileTaskBus {
    paths: TaskPaths,
}

impl LocalFileTaskBus {
    fn new(paths: TaskPaths) -> Self {
        Self { paths }
    }
}

impl TaskBus for LocalFileTaskBus {
    fn mode(&self) -> &'static str {
        "local"
    }

    fn enqueue(&self, payload: &TaskPayload) -> Result<(), String> {
        ensure_task_state(&self.paths)?;
        let row =
            serde_json::to_value(payload).map_err(|err| format!("local_enqueue_encode:{err}"))?;
        append_jsonl(&self.paths.queue_jsonl, &row)
    }

    fn dequeue(&self, max_messages: usize, _wait_ms: u64) -> Result<Vec<TaskPayload>, String> {
        ensure_task_state(&self.paths)?;
        let rows = read_queue_rows(&self.paths.queue_jsonl)?;
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let take = max_messages.min(rows.len());
        let selected = rows.iter().take(take).cloned().collect::<Vec<_>>();
        let remainder = rows.iter().skip(take).cloned().collect::<Vec<_>>();
        write_queue_rows(&self.paths.queue_jsonl, &remainder)?;
        Ok(selected)
    }

    fn publish_cancel(&self, _task_id: &str) -> Result<(), String> {
        Ok(())
    }

    fn pull_cancelled(&self, _max_messages: usize, _wait_ms: u64) -> Result<Vec<String>, String> {
        Ok(Vec::new())
    }
}

struct NatsTaskBus {
    runtime: Runtime,
    nats_url: String,
    stream_name: String,
    task_subject: String,
    cancel_subject: String,
    durable_name: String,
    cancel_durable_name: String,
}

impl NatsTaskBus {
    fn new(
        nats_url: String,
        stream_name: String,
        task_subject: String,
        cancel_subject: String,
        durable_name: String,
    ) -> Result<Self, String> {
        let runtime = Runtime::new().map_err(|err| format!("tokio_runtime_create_failed:{err}"))?;
        let cancel_durable_name = format!("{durable_name}-cancel");
        let bus = Self {
            runtime,
            nats_url,
            stream_name,
            task_subject,
            cancel_subject,
            durable_name,
            cancel_durable_name,
        };
        bus.ensure_stream()?;
        Ok(bus)
    }

    fn ensure_stream(&self) -> Result<(), String> {
        let nats_url = self.nats_url.clone();
        let stream_name = self.stream_name.clone();
        let task_subject = self.task_subject.clone();
        let cancel_subject = self.cancel_subject.clone();
        self.runtime.block_on(async move {
            let client = async_nats::connect(nats_url)
                .await
                .map_err(|err| format!("nats_connect_failed:{err}"))?;
            let context = jetstream::new(client);
            let config = StreamConfig {
                name: stream_name,
                subjects: vec![task_subject, cancel_subject],
                retention: jetstream::stream::RetentionPolicy::WorkQueue,
                max_messages: 100_000,
                ..Default::default()
            };
            context
                .get_or_create_stream(config)
                .await
                .map_err(|err| format!("nats_stream_create_failed:{err}"))?;
            Ok::<(), String>(())
        })
    }
}

impl TaskBus for NatsTaskBus {
    fn mode(&self) -> &'static str {
        "nats_jetstream"
    }

    fn enqueue(&self, payload: &TaskPayload) -> Result<(), String> {
        let raw =
            serde_json::to_vec(payload).map_err(|err| format!("nats_enqueue_encode:{err}"))?;
        let nats_url = self.nats_url.clone();
        let subject = self.task_subject.clone();
        self.runtime.block_on(async move {
            let client = async_nats::connect(nats_url)
                .await
                .map_err(|err| format!("nats_connect_failed:{err}"))?;
            let context = jetstream::new(client);
            let ack = context
                .publish(subject, raw.into())
                .await
                .map_err(|err| format!("nats_publish_failed:{err}"))?;
            ack.await
                .map_err(|err| format!("nats_publish_ack_failed:{err}"))?;
            Ok::<(), String>(())
        })
    }

    fn dequeue(&self, max_messages: usize, wait_ms: u64) -> Result<Vec<TaskPayload>, String> {
        let nats_url = self.nats_url.clone();
        let stream_name = self.stream_name.clone();
        let durable_name = self.durable_name.clone();
        let task_subject = self.task_subject.clone();
        self.runtime.block_on(async move {
            let client = async_nats::connect(nats_url)
                .await
                .map_err(|err| format!("nats_connect_failed:{err}"))?;
            let context = jetstream::new(client);
            let stream = context
                .get_stream(stream_name)
                .await
                .map_err(|err| format!("nats_get_stream_failed:{err}"))?;
            let consumer = stream
                .get_or_create_consumer(
                    durable_name.as_str(),
                    pull::Config {
                        durable_name: Some(durable_name.clone()),
                        filter_subject: task_subject,
                        ..Default::default()
                    },
                )
                .await
                .map_err(|err| format!("nats_consumer_create_failed:{err}"))?;
            let mut messages = consumer
                .fetch()
                .max_messages(max_messages)
                .expires(Duration::from_millis(wait_ms))
                .messages()
                .await
                .map_err(|err| format!("nats_fetch_failed:{err}"))?;
            let mut out = Vec::<TaskPayload>::new();
            while let Some(next) = messages.next().await {
                let message = next.map_err(|err| format!("nats_message_failed:{err}"))?;
                let parsed = serde_json::from_slice::<TaskPayload>(&message.payload)
                    .map_err(|err| format!("nats_payload_parse_failed:{err}"))?;
                message
                    .ack()
                    .await
                    .map_err(|err| format!("nats_ack_failed:{err}"))?;
                out.push(parsed);
            }
            Ok::<Vec<TaskPayload>, String>(out)
        })
    }

    fn publish_cancel(&self, task_id: &str) -> Result<(), String> {
        let nats_url = self.nats_url.clone();
        let subject = self.cancel_subject.clone();
        let payload = TaskCancelEnvelope {
            task_id: clean_id(task_id),
            ts_ms: now_epoch_ms(),
        };
        let raw =
            serde_json::to_vec(&payload).map_err(|err| format!("nats_cancel_encode:{err}"))?;
        self.runtime.block_on(async move {
            let client = async_nats::connect(nats_url)
                .await
                .map_err(|err| format!("nats_connect_failed:{err}"))?;
            let context = jetstream::new(client);
            let ack = context
                .publish(subject, raw.into())
                .await
                .map_err(|err| format!("nats_cancel_publish_failed:{err}"))?;
            ack.await
                .map_err(|err| format!("nats_cancel_publish_ack_failed:{err}"))?;
            Ok::<(), String>(())
        })
    }

    fn pull_cancelled(&self, max_messages: usize, wait_ms: u64) -> Result<Vec<String>, String> {
        let nats_url = self.nats_url.clone();
        let stream_name = self.stream_name.clone();
        let durable_name = self.cancel_durable_name.clone();
        let cancel_subject = self.cancel_subject.clone();
        self.runtime.block_on(async move {
            let client = async_nats::connect(nats_url)
                .await
                .map_err(|err| format!("nats_connect_failed:{err}"))?;
            let context = jetstream::new(client);
            let stream = context
                .get_stream(stream_name)
                .await
                .map_err(|err| format!("nats_get_stream_failed:{err}"))?;
            let consumer = stream
                .get_or_create_consumer(
                    durable_name.as_str(),
                    pull::Config {
                        durable_name: Some(durable_name.clone()),
                        filter_subject: cancel_subject,
                        ..Default::default()
                    },
                )
                .await
                .map_err(|err| format!("nats_cancel_consumer_create_failed:{err}"))?;
            let mut messages = consumer
                .fetch()
                .max_messages(max_messages)
                .expires(Duration::from_millis(wait_ms))
                .messages()
                .await
                .map_err(|err| format!("nats_cancel_fetch_failed:{err}"))?;
            let mut ids = Vec::<String>::new();
            while let Some(next) = messages.next().await {
                let message = next.map_err(|err| format!("nats_cancel_message_failed:{err}"))?;
                if let Ok(parsed) = serde_json::from_slice::<TaskCancelEnvelope>(&message.payload) {
                    let clean = clean_id(&parsed.task_id);
                    if !clean.is_empty() && !ids.iter().any(|row| row == &clean) {
                        ids.push(clean);
                    }
                }
                message
                    .ack()
                    .await
                    .map_err(|err| format!("nats_cancel_ack_failed:{err}"))?;
            }
            Ok::<Vec<String>, String>(ids)
        })
    }
}

fn build_task_bus(
    root: &Path,
    flags: &BTreeMap<String, String>,
) -> (Box<dyn TaskBus>, Vec<String>) {
    let paths = task_paths(root);
    let mut notes = Vec::<String>::new();
    let explicit_bus = parse_non_empty(flags, "bus")
        .or_else(|| std::env::var(TASK_BUS_ENV).ok())
        .unwrap_or_else(|| "auto".to_string())
        .to_ascii_lowercase();
    if explicit_bus == "local" {
        return (Box::new(LocalFileTaskBus::new(paths)), notes);
    }

    let nats_url =
        std::env::var(TASK_NATS_URL_ENV).unwrap_or_else(|_| DEFAULT_NATS_URL.to_string());
    let stream_name =
        std::env::var(TASK_NATS_STREAM_ENV).unwrap_or_else(|_| DEFAULT_NATS_STREAM.to_string());
    let task_subject =
        std::env::var(TASK_NATS_SUBJECT_ENV).unwrap_or_else(|_| DEFAULT_NATS_SUBJECT.to_string());
    let cancel_subject = std::env::var(TASK_NATS_CANCEL_SUBJECT_ENV)
        .unwrap_or_else(|_| DEFAULT_NATS_CANCEL_SUBJECT.to_string());
    let durable_name =
        std::env::var(TASK_NATS_DURABLE_ENV).unwrap_or_else(|_| DEFAULT_NATS_DURABLE.to_string());

    if explicit_bus == "nats" || explicit_bus == "auto" {
        match NatsTaskBus::new(
            nats_url,
            stream_name,
            task_subject,
            cancel_subject,
            durable_name,
        ) {
            Ok(bus) => return (Box::new(bus), notes),
            Err(err) => {
                notes.push(format!("task_bus_fallback_local:{err}"));
            }
        }
    }
    (Box::new(LocalFileTaskBus::new(paths)), notes)
}
