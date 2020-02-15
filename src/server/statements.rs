pub fn create_index_table (schema: &str, table: &str) -> String {
    format!(r#"
        CREATE TABLE "{}"."{}" (
            index tsvector NOT NULL,
            geo geography,
            result bytea NOT NULL
        )
    "#, schema, table)
}

pub fn add_entry (schema: &str, table: &str) -> String {
    format!(r#"
        INSERT INTO "{}"."{}" (index, geo, result)
            VALUES (
                json_to_tsvector('english', ($1::text)::json, '["string", "numeric"]'),
                ST_GeomFromEWKT($2)::geography,
                $3
            )
    "#, schema, table)
}

pub fn rename_table (schema: &str, from_table: &str, to_table: &str) -> String {
    format!(r#"ALTER TABLE "{}"."{}" RENAME TO "{}""#, schema, from_table, to_table)
}

pub fn drop_table (schema: &str, table: &str) -> String {
    format!(r#"DROP TABLE "{}"."{}""#, schema, table)
}


/*
$1 => q
$2 => lat
$3 => long
$4 => radius
*/
pub fn search (schema: &str, table: &str) -> String {
    format!(r#"
        SELECT result
            FROM "{}"."{}"
            WHERE ST_DWithin(geo, ST_SetSRID(ST_MakePoint($2::real, $3::real), 4326)::geography, $4::real, TRUE) IS NOT FALSE
            ORDER BY ts_rank(index,  websearch_to_tsquery('english', $1))
            LIMIT 10
    "#, schema, table)
}
