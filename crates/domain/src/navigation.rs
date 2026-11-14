//! Presentation pages and stable cursors, independent of answer storage.
use crate::{Definition, ValidationIssue};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PageFormat {
    #[default]
    Group,
    Question,
    All,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NavigationSettings {
    pub format: PageFormat,
    pub show_progress: bool,
}

impl Default for NavigationSettings {
    fn default() -> Self {
        Self {
            format: PageFormat::Group,
            show_progress: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub id: String,
    pub group_id: Option<String>,
    pub question_ids: Vec<String>,
}

impl NavigationSettings {
    pub fn pages(&self, definition: &Definition, relevant: &BTreeSet<String>) -> Vec<Page> {
        let mut pages = Vec::new();
        for group in &definition.groups {
            let mut question_ids = Vec::new();
            for question in &group.questions {
                let hidden = question.attributes.iter().any(|attribute| {
                    attribute.name == "hidden"
                        && attribute.value == "1"
                        && attribute.language.as_deref().is_none_or(str::is_empty)
                });
                if relevant.contains(&question.id) && !hidden {
                    question_ids.push(question.id.clone());
                }
            }
            match self.format {
                PageFormat::Question => {
                    for id in question_ids {
                        pages.push(Page {
                            id: format!("q:{id}"),
                            group_id: Some(group.id.clone()),
                            question_ids: vec![id],
                        });
                    }
                }
                PageFormat::Group | PageFormat::All if !question_ids.is_empty() => {
                    pages.push(Page {
                        id: format!("g:{}", group.id),
                        group_id: Some(group.id.clone()),
                        question_ids,
                    });
                }
                _ => {}
            }
        }
        if self.format == PageFormat::All {
            return vec![Page {
                id: "all".into(),
                group_id: None,
                question_ids: pages
                    .into_iter()
                    .flat_map(|page| page.question_ids)
                    .collect(),
            }];
        }
        if pages.is_empty() {
            pages.push(Page {
                id: "empty".into(),
                group_id: None,
                question_ids: Vec::new(),
            });
        }
        pages
    }

    /// Validate the requested identity, then recover from a page hidden by the new answers.
    pub fn resolve_page(
        &self,
        definition: &Definition,
        relevant: &BTreeSet<String>,
        page_id: Option<&str>,
        group_id: Option<&str>,
    ) -> Result<Page, ValidationIssue> {
        let pages = self.pages(definition, relevant);
        let invalid = || ValidationIssue {
            path: "checkpoint.pageId".into(),
            message: "Page inconnue ou incompatible avec le groupe et le format de la campagne."
                .into(),
        };
        let Some(id) = page_id else {
            if group_id.is_some() {
                return Err(invalid());
            }
            return Ok(pages[0].clone());
        };
        let known = match self.format {
            PageFormat::Group => id.strip_prefix("g:").is_some_and(|id| {
                definition
                    .groups
                    .iter()
                    .any(|group| group.id == id && group_id == Some(group.id.as_str()))
            }),
            PageFormat::Question => id.strip_prefix("q:").is_some_and(|id| {
                definition.groups.iter().any(|group| {
                    group_id == Some(group.id.as_str())
                        && group.questions.iter().any(|question| question.id == id)
                })
            }),
            PageFormat::All => id == "all" && group_id.is_none(),
        };
        if !known && !(id == "empty" && group_id.is_none() && pages[0].id == "empty") {
            return Err(invalid());
        }
        Ok(pages
            .iter()
            .find(|page| page.id == id)
            .unwrap_or(&pages[0])
            .clone())
    }
}
