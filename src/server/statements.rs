pub const CREATE: &str = r#"
CREATE TABLE index.test (
    index tsvector NOT NULL,
    geo geography,
    result bytea NOT NULL
)
"#;

pub const ADD_ENTRY: &str = r#"
INSERT INTO index.test (index, geo, result)
    VALUES (
        json_to_tsvector('english', ($1::text)::json, '["string", "numeric"]'),
        ST_GeomFromEWKT($2)::geography,
        $3
    )
"#;


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
