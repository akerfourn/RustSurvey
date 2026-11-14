use rustsurvey_domain::{Definition, SourceReference};
use rustsurvey_limesurvey::{
    Limits, LssDocument, SettingsContext, SourceId, SourceRuntimeFacts, export_native,
};
use serde_json::Value;

pub fn fixture() -> (Definition, String, SettingsContext, SourceRuntimeFacts) {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../../examples/rules/collection-forms.json"
    ))
    .unwrap();
    let mut definition: Definition = serde_json::from_value(corpus["definition"].clone()).unwrap();
    definition.upgrade().unwrap();
    let exported = export_native(&definition, SourceId(701), Limits::default()).unwrap();
    let mut xml = String::from_utf8(exported.xml).unwrap();
    xml = xml.replacen("<fieldname>sid</fieldname>", "<fieldname>sid</fieldname><fieldname>gsid</fieldname><fieldname>expires</fieldname><fieldname>startdate</fieldname><fieldname>emailresponseto</fieldname><fieldname>emailnotificationto</fieldname><fieldname>usecaptcha</fieldname>", 1)
        .replacen("<sid>701</sid>", "<sid>701</sid><gsid>2</gsid><usecaptcha>N</usecaptcha>", 1)
        .replace("<datestamp>N</datestamp>", "<datestamp>Y</datestamp>");
    for (name, value) in [
        ("showsurveypolicynotice", "0"),
        ("bounceprocessing", "N"),
        ("googleanalyticsstyle", "0"),
    ] {
        xml = xml
            .replacen(
                "<fieldname>sid</fieldname>",
                &format!("<fieldname>sid</fieldname><fieldname>{name}</fieldname>"),
                1,
            )
            .replacen(
                "<sid>701</sid>",
                &format!("<sid>701</sid><{name}>{value}</{name}>"),
                1,
            );
    }
    attach(
        &mut definition,
        &LssDocument::parse(xml.as_bytes(), Limits::default()).unwrap(),
    );
    let configuration: Value = serde_json::from_str(include_str!(
        "../../../../examples/rules/import-execution.json"
    ))
    .unwrap();
    let context = serde_json::from_value(configuration["context"].clone()).unwrap();
    let facts = serde_json::from_value(configuration["facts"].clone()).unwrap();
    (definition, xml, context, facts)
}

pub fn attach(definition: &mut Definition, document: &LssDocument) {
    let model = document.source_model().unwrap();
    for group in &mut definition.groups {
        group.source = Some(SourceReference {
            id: model.groups[0].id.0.to_string(),
            kind: None,
        });
        for question in &mut group.questions {
            let original = model
                .questions
                .iter()
                .find(|original| original.code == question.code)
                .unwrap();
            question.source = Some(SourceReference {
                id: original.id.0.to_string(),
                kind: Some(original.source_type.clone()),
            });
            question.theme = original.theme.clone();
            for row in &mut question.subquestions {
                let original = model
                    .subquestions
                    .iter()
                    .find(|row_source| {
                        row_source.parent_id == original.id
                            && row_source.code == row.code
                            && row_source.scale == row.scale
                    })
                    .unwrap();
                row.source = Some(SourceReference {
                    id: original.id.0.to_string(),
                    kind: None,
                });
            }
            for choice in &mut question.choices {
                let original = model
                    .answers
                    .iter()
                    .find(|answer| {
                        answer.question_id == original.id
                            && answer.code == choice.code
                            && answer.scale == choice.scale
                    })
                    .unwrap();
                choice.source = Some(SourceReference {
                    id: original.id.0.to_string(),
                    kind: None,
                });
            }
        }
    }
}
