                .unwrap_or(""),
            "confirmed"
        );
    }

    #[test]
    fn uploads_create_list_detail_delete() {
        let root = tempfile::tempdir().expect("tempdir");
        let created = handle(
            root.path(),
            "POST",
            "/api/uploads",
            "/api/uploads",
            &[],
            br#"{"filename":"notes.txt","content":"hello"}"#,
            &json!({}),
        )
        .expect("upload create");
        assert_eq!(created.status, 200);
        let upload_id = clean_text(
            created
                .payload
                .pointer("/upload/upload_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        assert!(!upload_id.is_empty());

        let listed = handle(
            root.path(),
            "GET",
            "/api/uploads",
            "/api/uploads",
            &[],
            &[],
            &json!({}),
        )
        .expect("uploads list");
        let rows = listed
            .payload
            .get("uploads")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(rows.len(), 1);

        let detail = handle(
            root.path(),
            "GET",
            &format!("/api/uploads/{upload_id}"),
            &format!("/api/uploads/{upload_id}"),
            &[],
            &[],
            &json!({}),
        )
        .expect("upload detail");
        assert_eq!(
            clean_text(
                detail
                    .payload
                    .pointer("/upload/upload_id")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                120
            ),
            upload_id
        );

        let deleted = handle(
            root.path(),
            "DELETE",
            &format!("/api/uploads/{upload_id}"),
            &format!("/api/uploads/{upload_id}"),
            &[],
            &[],
            &json!({}),
        )
        .expect("upload delete");
        assert!(deleted
            .payload
            .get("deleted")
            .and_then(Value::as_bool)
            .unwrap_or(false));
    }

    #[test]
    fn marketplace_aliases_and_reload_work() {
        let root = tempfile::tempdir().expect("tempdir");
        let browse = handle(
            root.path(),
            "GET",
            "/api/marketplace?limit=5",
            "/api/marketplace",
            &[],
            &[],
            &json!({}),
        )
        .expect("marketplace browse");
        assert_eq!(browse.status, 200);
        assert!(browse
            .payload
            .get("items")
            .map(Value::is_array)
            .unwrap_or(false));

        let reload = handle(
            root.path(),
            "POST",
            "/api/skills/reload",
            "/api/skills/reload",
            &[],
            &[],
            &json!({}),
        )
        .expect("skills reload");
        assert_eq!(reload.status, 200);
        assert!(reload
            .payload
            .get("reloaded")
            .and_then(Value::as_bool)
            .unwrap_or(false));
    }
}
