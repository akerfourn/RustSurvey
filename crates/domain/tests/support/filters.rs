use rustsurvey_domain::Definition;
use serde_json::{Value, json};

pub fn cases() -> Vec<Value> {
    serde_json::from_str::<Value>(include_str!("../../../../examples/rules/filters.json")).unwrap()
        ["cases"]
        .as_array()
        .unwrap()
        .clone()
}

fn question(base: &Value, kind: &str, code: &str) -> Value {
    let mut question = base["groups"][0]["questions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|question| question["kind"] == kind)
        .unwrap()
        .clone();
    question["id"] = json!(code);
    question["code"] = json!(code);
    question["mandatory"] = json!("no");
    question["relevance"] = json!("1");
    question["other"] = json!(false);
    question["attributes"] = json!([]);
    for axis in ["subquestions", "choices"] {
        for (index, item) in question[axis]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .enumerate()
        {
            item["id"] = json!(format!("{code}-{axis}-{index}"));
            item["relevance"] = json!("1");
        }
    }
    if matches!(kind, "ranking" | "single_choice") {
        for (index, choice) in question["choices"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .enumerate()
        {
            choice["code"] = json!(format!("A{}", index + 1));
        }
    }
    question
}

fn rows(question: &mut Value, codes: &Value) {
    if let Some(codes) = codes.as_array() {
        let prototype = question["subquestions"][0].clone();
        question["subquestions"] = json!(
            codes
                .iter()
                .enumerate()
                .map(|(index, code)| {
                    let mut row = prototype.clone();
                    row["id"] = json!(format!(
                        "{}-row-{index}",
                        question["code"].as_str().unwrap()
                    ));
                    row["code"] = code.clone();
                    row
                })
                .collect::<Vec<_>>()
        );
    }
}

pub fn definition(case: &Value) -> Definition {
    let mut base: Value = serde_json::from_str::<Value>(include_str!(
        "../../../../examples/rules/collection-forms.json"
    ))
    .unwrap()["definition"]
        .clone();
    let mut source = question(&base, case["kind"].as_str().unwrap(), "SRC");
    rows(&mut source, &case["sourceRows"]);
    source["other"] = json!(case["other"] == true);
    if let Some(settings) = case["sourceAttributes"].as_object() {
        for (name, value) in settings {
            source["attributes"]
                .as_array_mut()
                .unwrap()
                .push(json!({"name":name,"value":value}));
        }
    }
    for row in source["subquestions"].as_array_mut().unwrap() {
        if case["sourceHiddenRows"]
            .as_array()
            .is_some_and(|hidden| hidden.contains(&row["code"]))
        {
            row["relevance"] = json!("0");
        }
    }
    let mut questions = vec![source];
    let mut filter_source = "SRC";
    if case["middleRows"].is_array() || case["middleKind"].is_string() {
        let mut middle = question(
            &base,
            case["middleKind"].as_str().unwrap_or("multiple_text"),
            "MID",
        );
        rows(&mut middle, &case["middleRows"]);
        let name = if case["ancestorMode"] == "exclude" {
            "array_filter_exclude"
        } else {
            "array_filter"
        };
        middle["attributes"] = json!([{"name":name,"value":"SRC"}]);
        questions.push(middle);
        filter_source = "MID";
    }
    let mut target = question(
        &base,
        case["targetKind"].as_str().unwrap_or("array_choice"),
        "TARGET",
    );
    rows(&mut target, &case["targetRows"]);
    if let Some(codes) = case["targetChoices"].as_array() {
        let prototype = target["choices"][0].clone();
        target["choices"] = json!(
            codes
                .iter()
                .enumerate()
                .map(|(index, code)| {
                    let mut choice = prototype.clone();
                    choice["id"] = json!(format!("TARGET-choice-{index}"));
                    choice["code"] = code.clone();
                    choice
                })
                .collect::<Vec<_>>()
        );
    }
    target["other"] = json!(case["targetOther"] == true);
    let name = if case["mode"] == "exclude" {
        "array_filter_exclude"
    } else {
        "array_filter"
    };
    target["attributes"] = json!([{"name":name,"value":filter_source},{"name":"array_filter_style","value":case["style"].as_str().unwrap_or("0")}]);
    questions.push(target);
    base["groups"][0]["questions"] = json!(questions);
    serde_json::from_value(base).unwrap()
}

pub fn answers(case: &Value, definition: &Definition) -> Value {
    let mut answers = case["answers"].as_object().unwrap().clone();
    if let Some(target) = case["targetAnswers"].as_object() {
        answers.extend(target.clone());
        return json!(answers);
    }
    let target = definition.groups[0].questions.last().unwrap();
    let comment = target.kind == rustsurvey_domain::QuestionKind::MultipleChoiceComment;
    for row in &target.subquestions {
        let value = if comment { "Y" } else { "01" };
        answers.insert(format!("TARGET_{}", row.code), json!(value));
        if comment {
            answers.insert(format!("TARGET_{}comment", row.code), json!("note"));
        }
    }
    if target.other {
        answers.insert("TARGET_other".into(), json!("custom"));
        if comment {
            answers.insert("TARGET_othercomment".into(), json!("note"));
        }
    }
    json!(answers)
}
