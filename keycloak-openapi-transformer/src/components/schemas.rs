use openapiv3::ObjectType;
use openapiv3::Schema;
use openapiv3::SchemaKind;
use scraper::Selector;
use std::collections::BTreeMap;

pub fn parse_schemas(
    document: &scraper::html::Html,
) -> BTreeMap<String, openapiv3::ReferenceOr<Schema>> {
    let schemas_selector = Selector::parse("#_definitions + .sectionbody > .sect2").unwrap();
    let title_selector = Selector::parse("h3").unwrap();
    document
        .select(&schemas_selector)
        .map(|section| {
            (
                section
                    .select(&title_selector)
                    .next()
                    .unwrap()
                    .text()
                    .collect(),
                openapiv3::ReferenceOr::Item(parse_schema(section)),
            )
        })
        .collect()
}

fn enum_type(raw_type: &str) -> Option<openapiv3::Type> {
    const START: &str = "enum (";
    const END: &str = ")";
    if raw_type.starts_with(START) && raw_type.ends_with(END) {
        let enumerations = raw_type
            .get(START.len()..raw_type.len() - END.len())?
            .split(", ")
            .map(std::string::ToString::to_string)
            .collect();
        Some(openapiv3::Type::String(openapiv3::StringType {
            enumeration: enumerations,
            ..Default::default()
        }))
    } else {
        None
    }
}

fn parse_type(raw_type: &str) -> openapiv3::ReferenceOr<Box<Schema>> {
    let schema_type = enum_type(&raw_type).unwrap_or_else(|| match raw_type {
        "integer(int32)" => openapiv3::Type::Integer(openapiv3::IntegerType {
            format: openapiv3::VariantOrUnknownOrEmpty::Item(openapiv3::IntegerFormat::Int32),
            ..Default::default()
        }),
        "integer(int64)" => openapiv3::Type::Integer(openapiv3::IntegerType {
            format: openapiv3::VariantOrUnknownOrEmpty::Item(openapiv3::IntegerFormat::Int64),
            ..Default::default()
        }),
        "number(float)" => openapiv3::Type::Number(openapiv3::NumberType {
            format: openapiv3::VariantOrUnknownOrEmpty::Item(openapiv3::NumberFormat::Float),
            ..Default::default()
        }),
        "boolean" => openapiv3::Type::Boolean {},
        "< string > array" => openapiv3::Type::Array(openapiv3::ArrayType {
            items: openapiv3::ReferenceOr::Item(Box::new(Schema {
                schema_data: Default::default(),
                schema_kind: SchemaKind::Type(openapiv3::Type::String(Default::default())),
            })),
            min_items: None,
            max_items: None,
            unique_items: false,
        }),
        "Map" => openapiv3::Type::Object(Default::default()),
        "Object" => openapiv3::Type::Object(Default::default()),
        _ => openapiv3::Type::String(Default::default()),
    });

    openapiv3::ReferenceOr::Item(Box::new(Schema {
        schema_data: Default::default(),
        schema_kind: SchemaKind::Type(schema_type),
    }))
}

fn parse_schema(section: scraper::element_ref::ElementRef<'_>) -> Schema {
    let row_selector = Selector::parse("table > tbody > tr").unwrap();
    let property_name_selector = Selector::parse("td:first-child strong").unwrap();
    let type_selector = Selector::parse("td:first-child + td").unwrap();
    let properties = section
        .select(&row_selector)
        .map(|row| {
            (
                row.select(&property_name_selector)
                    .next()
                    .unwrap()
                    .text()
                    .collect::<String>(),
                parse_type(
                    &row.select(&type_selector)
                        .next()
                        .unwrap()
                        .text()
                        .collect::<String>(),
                ),
            )
        })
        .collect();
    Schema {
        schema_data: Default::default(),
        schema_kind: SchemaKind::Type(openapiv3::Type::Object(ObjectType {
            properties,
            ..Default::default()
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_schemas;
    use openapiv3::OpenAPI;
    use scraper::Html;

    const HTML: &str = include_str!("../../../keycloak/6.0.html");
    const JSON: &str = include_str!("../../../keycloak/6.0.json");

    fn parse_schema_correctly(schema: &str) {
        let openapi: OpenAPI = serde_json::from_str(JSON).expect("Could not deserialize example");
        let components = openapi.components.expect("Couldn't deserialize components");

        assert_eq!(
            components.schemas.get(schema),
            parse_schemas(&Html::parse_document(HTML)).get(schema)
        );
    }

    #[test]
    fn parses_string_only_schema_as_expected() {
        parse_schema_correctly("AccessToken-CertConf");
    }

    #[test]
    fn parses_int32_only_schema_as_expected() {
        parse_schema_correctly("ClientInitialAccessCreatePresentation");
    }

    #[test]
    fn parses_schema_with_bool_as_expected() {
        parse_schema_correctly("SynchronizationResult");
    }

    #[test]
    fn parses_schema_with_float_as_expected() {
        parse_schema_correctly("MultivaluedHashMap");
    }

    #[test]
    fn parses_schema_with_int64_as_expected() {
        parse_schema_correctly("MemoryInfoRepresentation");
    }

    #[test]
    fn parses_schema_only_map_as_expected() {
        parse_schema_correctly("SpiInfoRepresentation");
    }

    #[test]
    fn parses_schema_with_enum_as_expected() {
        parse_schema_correctly("PolicyRepresentation");
    }

    #[test]
    fn parses_schema_with_object_as_expected() {
        parse_schema_correctly("ConfigPropertyRepresentation");
    }
}
