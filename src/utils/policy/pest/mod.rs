use pest::Parser;
use std::string::String;
use RabeError;

pub(crate) mod json;
pub(crate) mod human;
use self::human::HumanPolicyParser;
use self::json::JSONPolicyParser;

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum PolicyLanguage {
    JsonPolicy,
    HumanPolicy,
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub enum PolicyType {
    And,
    Or,
    Leaf
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub enum PolicyValue<'a> {
    Object((PolicyType, Box<PolicyValue<'a>>)),
    Array(Vec<PolicyValue<'a>>),
    String(&'a str),
}

pub fn parse(policy: &str, language: PolicyLanguage) -> Result<PolicyValue, RabeError> {
    match language {
        PolicyLanguage::JsonPolicy => {
            use utils::policy::pest::json::Rule;
            match JSONPolicyParser::parse(Rule::content, policy) {
                Ok(mut result) => Ok(json::parse(result.next().unwrap())),
                Err(e) => Err(e.into())
            }
        },
        PolicyLanguage::HumanPolicy => {
            use utils::policy::pest::human::Rule;
            match HumanPolicyParser::parse(Rule::content, policy) {
                Ok(mut result) => Ok(human::parse(result.next().unwrap())),
                Err(e) => Err(e.into())
            }
        }
    }
}

pub fn serialize_policy(val: &PolicyValue, language: PolicyLanguage, parent: Option<PolicyType>) -> String {
    use self::PolicyValue::*;
    match language {
        PolicyLanguage::JsonPolicy => {
            match val {
                Object(obj) => {
                    match obj.0 {
                        PolicyType::And => format!("{{\"name\": \"and\", {}}}", serialize_policy(obj.1.as_ref(), language, None)),
                        PolicyType::Or => format!("{{\"name\": \"or\", {}}}", serialize_policy(obj.1.as_ref(), language, None)),
                        PolicyType::Leaf => serialize_policy(&obj.1.as_ref(), language, None)
                    }
                },
                Array(a) => {
                    let contents: Vec<_> = a.iter().map(|val| serialize_policy(val, language, None)).collect();
                    format!("\"children\": [{}]", contents.join(", "))
                }
                String(s) => format!("{{\"name\": \"{}\"}}", s),
            }
        },
        PolicyLanguage::HumanPolicy => {
            match val {
                Object(obj) => {
                    match obj.0 {
                        PolicyType::And => format!("{}", serialize_policy(obj.1.as_ref(), language, Some(PolicyType::And))),
                        PolicyType::Or => format!("{}", serialize_policy(obj.1.as_ref(), language, Some(PolicyType::Or))),
                        PolicyType::Leaf => serialize_policy(&obj.1.as_ref(), language, Some(PolicyType::Leaf))
                    }
                },
                Array(a) => {
                    let contents: Vec<_> = a.iter().map(|val| serialize_policy(val, language, None)).collect();
                    match parent {
                        Some(PolicyType::And) => format!("({})", contents.join(" and ")),
                        Some(PolicyType::Or) => format!("({})", contents.join(" or ")),
                        _ => panic!("children without parent")
                    }
                }
                String(s) => format!("{}", s),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_parsing() {
        let pol = String::from(r#"{"name": "A"}"#);
        let human = String::from("A");
        let json: PolicyValue = parse(&pol, PolicyLanguage::JsonPolicy).expect("unsuccessful parse");
        let serialized_json = serialize_policy(&json, PolicyLanguage::JsonPolicy, None);
        let serialized_human = serialize_policy(&json, PolicyLanguage::HumanPolicy, None);
        assert_eq!(serialized_json, pol);
        assert_eq!(serialized_human, human);
    }

    #[test]
    fn test_children_parsing() {
        let pol = String::from(r#"{"name": "and", "children": [{"name": "B"}, {"name": "C"}]}"#);
        let human = String::from("(B and C)");
        let json: PolicyValue = parse(&pol, PolicyLanguage::JsonPolicy).expect("unsuccessful parse");
        let serialized_json =serialize_policy(&json, PolicyLanguage::JsonPolicy, None);
        let serialized_human =serialize_policy(&json, PolicyLanguage::HumanPolicy, None);
        assert_eq!(serialized_json, pol);
        assert_eq!(serialized_human, human);
    }

    #[test]
    fn test_sub_children_parsing() {
        let pol = String::from(r#"{"name": "or", "children": [{"name": "A"}, {"name": "and", "children": [{"name": "B"}, {"name": "C"}]}]}"#);
        let human = String::from(r#"(A or (B and C))"#);
        let json: PolicyValue = parse(&pol, PolicyLanguage::JsonPolicy).expect("unsuccessful parse");
        let serialized_json =serialize_policy(&json, PolicyLanguage::JsonPolicy, None);
        let serialized_human =serialize_policy(&json, PolicyLanguage::HumanPolicy, None);
        assert_eq!(serialized_json, pol);
        assert_eq!(serialized_human, human);
    }
}