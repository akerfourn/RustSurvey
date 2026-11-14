use rustsurvey_domain::{
    Definition,
    collection::{Answers, PAGES_PROFILE, Questionnaire},
    navigation::{NavigationSettings, PageFormat},
};
use serde::Deserialize;
use serde_json::Value;

fn definition() -> Definition {
    let corpus: Value =
        serde_json::from_str(include_str!("../../../examples/rules/collection.json")).unwrap();
    serde_json::from_value(corpus["definition"].clone()).unwrap()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Case {
    name: String,
    format: PageFormat,
    answers: Answers,
    pages: Value,
    #[serde(default)]
    hide_all: bool,
    #[serde(default)]
    reverse_groups: bool,
}

#[test]
fn test_navigation_should_match_the_shared_page_corpus_in_all_three_formats() {
    #[derive(Deserialize)]
    struct Corpus {
        cases: Vec<Case>,
    }
    let corpus: Corpus =
        serde_json::from_str(include_str!("../../../examples/rules/navigation.json")).unwrap();
    for case in corpus.cases {
        let mut definition = definition();
        if case.hide_all {
            for group in &mut definition.groups {
                group.relevance = "0".into();
            }
        }
        if case.reverse_groups {
            definition.groups.reverse();
        }
        let runtime = Questionnaire::compile_profile(&definition, PAGES_PROFILE).unwrap();
        let snapshot = runtime.evaluate(&case.answers, false).unwrap();
        for show_progress in [true, false] {
            let settings = NavigationSettings {
                format: case.format,
                show_progress,
            };
            let actual =
                serde_json::to_value(settings.pages(&definition, &snapshot.relevant)).unwrap();
            assert_eq!(actual, case.pages, "{}", case.name);
        }
    }
}

#[test]
fn test_page_cursor_should_reject_cross_format_or_cross_group_ids_and_recover_hidden_pages() {
    let definition = definition();
    let runtime = Questionnaire::compile_forms(&definition).unwrap();
    let snapshot = runtime.evaluate(&Answers::new(), false).unwrap();
    let settings = NavigationSettings {
        format: PageFormat::Question,
        show_progress: false,
    };
    let recovered = settings
        .resolve_page(
            &definition,
            &snapshot.relevant,
            Some("q:age"),
            Some("details"),
        )
        .unwrap();
    assert_eq!(recovered.id, "q:gate");
    for (page, group) in [
        (Some("q:age"), Some("main")),
        (Some("g:details"), Some("details")),
        (Some("q:AGE"), Some("details")),
        (Some("all"), None),
        (Some("empty"), None),
        (None, Some("main")),
    ] {
        let issue = settings
            .resolve_page(&definition, &snapshot.relevant, page, group)
            .unwrap_err();
        assert_eq!(issue.path, "checkpoint.pageId");
        assert!(issue.message.contains("incompatible"));
    }
}
