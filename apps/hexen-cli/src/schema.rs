//! A JSON Schema for .hexen.yml, derived from schema/hexen/v1/*.proto
//! (the compiled descriptors hexen-proto embeds) rather than maintained
//! by hand — replaces packages/hexen-schema's Zod-generated one, which
//! described the old "type:" format. Mirrors pbjson's JSON mapping:
//! lowerCamelCase field names, a oneof as `{ <member>: value }`, unknown
//! fields rejected, `null` accepted for message and `optional` fields.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;

use prost::Message;
use prost_types::field_descriptor_proto::{Label, Type};
use prost_types::{DescriptorProto, FieldDescriptorProto, FileDescriptorSet};
use serde_json::{json, Map, Value};

const ROOT: &str = ".hexen.v1.HexenProject";

struct Schema {
    messages: HashMap<String, DescriptorProto>,
    /// Proto comment per message / "Message.field", from source info.
    docs: HashMap<String, String>,
}

impl Schema {
    fn load() -> Schema {
        let set = FileDescriptorSet::decode(hexen_proto::FILE_DESCRIPTOR_SET)
            .expect("hexen-proto's descriptor set decodes");
        let mut messages = HashMap::new();
        let mut docs = HashMap::new();
        for file in set.file {
            let package = file.package().to_string();
            // Source-info paths: [4, i] is the i-th top-level message,
            // [4, i, 2, j] its j-th field.
            let comments: HashMap<Vec<i32>, String> = file
                .source_code_info
                .iter()
                .flat_map(|info| &info.location)
                .filter_map(|loc| {
                    let text = loc.leading_comments().trim();
                    (!text.is_empty()).then(|| (loc.path.clone(), clean_comment(text)))
                })
                .collect();
            for (i, message) in file.message_type.iter().enumerate() {
                let name = format!(".{package}.{}", message.name());
                if let Some(doc) = comments.get(&vec![4, i as i32]) {
                    docs.insert(name.clone(), doc.clone());
                }
                for (j, field) in message.field.iter().enumerate() {
                    if let Some(doc) = comments.get(&vec![4, i as i32, 2, j as i32]) {
                        docs.insert(format!("{name}.{}", field.name()), doc.clone());
                    }
                }
                messages.insert(name, message.clone());
            }
        }
        Schema { messages, docs }
    }

    fn definition(&self, full_name: &str, out: &mut BTreeMap<String, Value>) {
        let short = short_name(full_name);
        if out.contains_key(short) {
            return;
        }
        out.insert(short.to_string(), Value::Null); // reserve; guards recursion
        let message = &self.messages[full_name];

        let mut properties = Map::new();
        for field in &message.field {
            let mut schema = self.field_schema(field, out);
            if let (Some(doc), Some(obj)) = (
                self.docs.get(&format!("{full_name}.{}", field.name())),
                schema.as_object_mut(),
            ) {
                obj.insert("description".into(), doc.clone().into());
            }
            properties.insert(json_name(field), schema);
        }

        let mut def =
            json!({ "type": "object", "properties": properties, "additionalProperties": false });
        if let Some(doc) = self.docs.get(full_name) {
            def["description"] = doc.clone().into();
        }

        // Real oneofs (not proto3 `optional`'s synthetic ones): at most one
        // member may be set, and a message that's nothing but one oneof
        // needs exactly one to mean anything.
        let mut all_of = Vec::new();
        for (index, _) in message.oneof_decl.iter().enumerate() {
            let members: Vec<String> = message
                .field
                .iter()
                .filter(|f| f.oneof_index == Some(index as i32) && !f.proto3_optional())
                .map(json_name)
                .collect();
            if members.len() < 2 {
                continue;
            }
            let pairs: Vec<Value> = members
                .iter()
                .enumerate()
                .flat_map(|(i, a)| {
                    members[i + 1..]
                        .iter()
                        .map(move |b| json!({ "required": [a, b] }))
                })
                .collect();
            all_of.push(json!({ "not": { "anyOf": pairs } }));
            if members.len() == message.field.len() {
                def["minProperties"] = 1.into();
            }
        }
        if !all_of.is_empty() {
            def["allOf"] = all_of.into();
        }
        out.insert(short.to_string(), def);
    }

    fn field_schema(
        &self,
        field: &FieldDescriptorProto,
        out: &mut BTreeMap<String, Value>,
    ) -> Value {
        let item = match field.r#type() {
            Type::Double | Type::Float => json!({ "type": "number" }),
            Type::Int32 | Type::Uint32 | Type::Sint32 | Type::Fixed32 | Type::Sfixed32 => {
                json!({ "type": "integer" })
            }
            // pbjson writes 64-bit integers as strings, and reads either.
            Type::Int64 | Type::Uint64 | Type::Sint64 | Type::Fixed64 | Type::Sfixed64 => {
                json!({ "type": ["integer", "string"] })
            }
            Type::Bool => json!({ "type": "boolean" }),
            Type::String | Type::Bytes => json!({ "type": "string" }),
            Type::Enum => json!({ "type": "string" }),
            Type::Message | Type::Group => {
                self.definition(field.type_name(), out);
                json!({ "$ref": format!("#/definitions/{}", short_name(field.type_name())) })
            }
        };
        if field.label() == Label::Repeated {
            return json!({ "type": "array", "items": item });
        }
        if field.r#type() == Type::Message || field.proto3_optional() {
            return json!({ "anyOf": [item, { "type": "null" }] });
        }
        item
    }
}

fn short_name(full: &str) -> &str {
    full.rsplit('.').next().unwrap_or(full)
}

fn json_name(field: &FieldDescriptorProto) -> String {
    if let Some(name) = &field.json_name {
        return name.clone();
    }
    let mut out = String::new();
    let mut upper = false;
    for c in field.name().chars() {
        if c == '_' {
            upper = true;
        } else if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// A proto comment block as one paragraph-preserving string.
fn clean_comment(text: &str) -> String {
    text.split("\n\n")
        .map(|para| para.lines().map(str::trim).collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub fn generate() -> Value {
    let schema = Schema::load();
    let mut definitions = BTreeMap::new();
    schema.definition(ROOT, &mut definitions);
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Hex Enductor project (.hexen.yml)",
        "description": "Generated from schema/hexen/v1/*.proto by `cargo run -p hexen-cli -- schema`. Don't edit by hand.",
        "$ref": format!("#/definitions/{}", short_name(ROOT)),
        "definitions": definitions,
    })
}

pub fn render() -> String {
    format!(
        "{}\n",
        serde_json::to_string_pretty(&generate()).expect("a JSON value serializes")
    )
}

pub fn write(out: &Path) -> Result<(), String> {
    fs::write(out, render()).map_err(|e| format!("{}: {e}", out.display()))?;
    println!("Wrote {}", out.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validator() -> jsonschema::Validator {
        jsonschema::draft7::new(&generate()).expect("the generated schema is valid draft-07")
    }

    fn yaml_to_json(text: &str) -> Value {
        serde_yml::from_str(text).unwrap()
    }

    #[test]
    fn the_demo_project_validates() {
        let demo = include_str!("../../../examples/demo/demo.hexen.yml");
        let errors: Vec<String> = validator()
            .iter_errors(&yaml_to_json(demo))
            .map(|e| e.to_string())
            .collect();
        assert!(errors.is_empty(), "{errors:#?}");
    }

    #[test]
    fn the_old_type_format_and_mistakes_are_rejected() {
        let v = validator();
        let old = "schemaVersion: 1\ntitle: T\ndefaultLocation: a\ncontent: { type: inline }\nlocations: []\n";
        assert!(!v.is_valid(&yaml_to_json(old)));
        let both = "schemaVersion: 1\ntitle: T\ndefaultLocation: a\ncontent: { inline: {}, obsidian: { vaultRoot: . } }\n";
        assert!(
            !v.is_valid(&yaml_to_json(both)),
            "two oneof members at once"
        );
        let typo = "schemaVersion: 1\ntitle: T\ndefaultLocation: a\ncontent: { inline: {} }\nlocations: [{ id: a, imgae: null }]\n";
        assert!(!v.is_valid(&yaml_to_json(typo)), "unknown field");
    }

    #[test]
    fn carries_proto_comments_as_descriptions() {
        let schema = generate();
        let link_id = &schema["definitions"]["Link"]["properties"]["id"]["description"];
        assert!(
            link_id.as_str().unwrap().contains("stable across edits"),
            "{link_id}"
        );
    }

    #[test]
    fn the_checked_in_schema_is_current() {
        let checked_in = include_str!("../../../docs/hexen.schema.json");
        assert!(
            checked_in == render(),
            "docs/hexen.schema.json is stale — run `cargo run -p hexen-cli -- schema`"
        );
    }
}
