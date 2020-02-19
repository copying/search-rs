macro_rules! pgformat {
    ( $x:expr,$( $y:expr ),* ) => {
        format!($x, $($y.replace("\"", "\"\"")),*)
    }
}

pub const ADD_POSTGIS: &str = "CREATE EXTENSION IF NOT EXISTS postgis";

pub fn create_main_table (schema: &str, table: &str) -> String {
    pgformat!(r#"
        CREATE TABLE IF NOT EXISTS "{}"."{}" (
            name varchar(100) primary key,
            lang varchar(25) not null,
            response_size integer not null
        )
    "#, schema, table)
}

pub fn add_index (schema: &str, table: &str) -> String {
    pgformat!(r#"
        INSERT INTO "{}"."{}" (name, lang, response_size)
            VALUES ($1, $2, $3)
    "#, schema, table)
}

pub fn delete_index (schema: &str, table: &str) -> String {
    pgformat!(r#"
        DELETE FROM "{}"."{}"
            WHERE name = $1
    "#, schema, table)
}

pub fn get_index (schema: &str, table: &str) -> String {
    pgformat!(r#"
        SELECT lang, response_size
            FROM "{}"."{}"
            WHERE name = $1
            LIMIT 1
    "#, schema, table)
}

pub fn create_index_table (schema: &str, table: &str) -> String {
    pgformat!(r#"
        CREATE TABLE "{}"."{}" (
            index tsvector NOT NULL,
            geo geography,
            result bytea NOT NULL
        )
    "#, schema, table)
}

pub fn add_entry (schema: &str, table: &str) -> String {
    pgformat!(r#"
        INSERT INTO "{}"."{}" (index, geo, result)
            VALUES (
                json_to_tsvector($3, ($1::text)::json, '["string", "numeric"]'),
                ST_GeomFromEWKT($2)::geography,
                $3
            )
    "#, schema, table)
}

pub fn rename_table (schema: &str, from_table: &str, to_table: &str) -> String {
    pgformat!(r#"ALTER TABLE "{}"."{}" RENAME TO "{}""#, schema, from_table, to_table)
}

pub fn drop_table (schema: &str, table: &str) -> String {
    pgformat!(r#"DROP TABLE "{}"."{}""#, schema, table)
}


/*
$1 => q
$2 => lat
$3 => long
$4 => radius
*/
pub fn search (schema: &str, table: &str) -> String {
    pgformat!(r#"
        SELECT result
            FROM "{}"."{}"
            WHERE ST_DWithin(geo, ST_SetSRID(ST_MakePoint($2::real, $3::real), 4326)::geography, $4::real, TRUE) IS NOT FALSE
            ORDER BY ts_rank(index,  websearch_to_tsquery($3, $1))
            LIMIT $4
    "#, schema, table)
}
