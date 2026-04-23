#[cfg(test)]
mod health_tests {
    use super::*;

    #[test]
    fn dashboard_health_response_ok_accepts_2xx_status_codes() {
        assert!(dashboard_health_response_ok(
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{}"
        ));
        assert!(dashboard_health_response_ok(
            b"HTTP/1.1 204 No Content\r\nContent-Type: application/json\r\n\r\n"
        ));
    }

    #[test]
    fn dashboard_health_response_ok_rejects_non_2xx_status_codes() {
        assert!(!dashboard_health_response_ok(
            b"HTTP/1.1 503 Service Unavailable\r\nContent-Type: text/plain\r\n\r\noffline"
        ));
    }

    #[test]
    fn dashboard_web_tooling_response_ready_accepts_auth_signals() {
        assert!(dashboard_web_tooling_response_ready(
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"ok\":true,\"any_present\":true}"
        ));
        assert!(dashboard_web_tooling_response_ready(
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"ok\":true,\"readiness\":\"ready\"}"
        ));
    }

    #[test]
    fn dashboard_web_tooling_response_ready_rejects_missing_auth_signals() {
        assert!(!dashboard_web_tooling_response_ready(
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"ok\":true,\"auth_sources\":[]}"
        ));
    }
}
