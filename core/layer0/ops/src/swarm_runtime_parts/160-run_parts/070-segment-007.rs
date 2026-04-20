            append_event(
                &mut state,
                json!({
                    "type": "swarm_runtime_command",
                    "command": cmd,
                    "timestamp": now_iso(),
                    "ok": true,
                }),
            );
            let _ = save_state(&state_file, &state);
            print_receipt(payload);
            0
        }
        Err(err) => {
            print_receipt(json!({
                "ok": false,
                "type": "swarm_runtime_error",
                "command": cmd,
                "error": err,
                "state_path": state_file,
            }));
            2
        }
    }
}
