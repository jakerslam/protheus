fn fetch_strip_invisible_unicode(raw: &str) -> String {
    raw.chars()
        .filter(|ch| {
            !matches!(
                *ch,
                '\u{200B}'..='\u{200F}'
                    | '\u{202A}'..='\u{202E}'
                    | '\u{2060}'..='\u{2064}'
                    | '\u{206A}'..='\u{206F}'
                    | '\u{FEFF}'
                    | '\u{E0000}'..='\u{E007F}'
            )
        })
        .collect()
}
