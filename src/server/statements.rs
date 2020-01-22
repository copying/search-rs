pub const ADD_ENTRY: &str = r#"
INSERT INTO index.test (index, geom, result)
    VALUES (
        json_to_tsvector('english', ($1::text)::json, '["string", "numeric"]'),
        ST_GeomFromEWKT($2)::geometry,
        $3
    )
"#;
