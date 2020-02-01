pub fn create_index_table (table: &String) -> String {
    format!(r#"
        CREATE TABLE index."{}" (
            index tsvector NOT NULL,
            geo geography,
            result bytea NOT NULL
        )
    "#, table)
}

pub fn add_entry (table: &String) -> String {
    format!(r#"
        INSERT INTO index."{}" (index, geo, result)
            VALUES (
                json_to_tsvector('english', ($1::text)::json, '["string", "numeric"]'),
                ST_GeomFromEWKT($2)::geography,
                $3
            )
    "#, table)
}

pub fn rename_table (from_table: &String, to_table: &String) -> String {
    format!(r#"ALTER TABLE index."{}" RENAME TO "{}""#, from_table, to_table)
}

pub fn drop_table (table: &String) -> String {
    format!(r#"DROP TABLE index."{}""#, table)
}


/*
$1 => q
$2 => lat
$3 => long
$4 => radius
*/
pub const SEARCH: &str = r#"
SELECT result
    FROM index.test
    WHERE ST_DWithin(geo, ST_SetSRID(ST_MakePoint($2::real, $3::real), 4326)::geography, $4::real, TRUE) IS NOT FALSE
    ORDER BY ts_rank(index,  websearch_to_tsquery('english', $1))
    LIMIT 10
"#;
