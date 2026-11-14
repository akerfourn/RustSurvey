use super::{AppError, Checkpoint, Document, Run, Snapshot, WorkspaceState};
use axum::http::StatusCode;
use rustsurvey_domain::Definition;
use rustsurvey_domain::{collection::PageCursor, navigation::NavigationSettings};

pub(super) async fn initial(
    state: &WorkspaceState,
    run: &Run,
    definition: Definition,
    language: String,
) -> Result<Document, AppError> {
    let mut checkpoint = Checkpoint {
        language,
        group_id: None,
        page_id: None,
    };
    super::checkpoint(&definition, &checkpoint)?;
    let mut answers = Default::default();
    if run.navigation.is_some() {
        let (_, snapshot, initialized) =
            super::evaluate(state, definition, answers, false, run, None, &checkpoint).await?;
        checkpoint = initialized;
        answers = snapshot.answers;
    } else {
        checkpoint.group_id = definition.groups.first().map(|group| group.id.clone());
    }
    Ok(Document {
        answers,
        checkpoint,
    })
}

pub(super) fn normalize(
    settings: Option<NavigationSettings>,
    definition: &Definition,
    snapshot: &Snapshot,
    checkpoint: &Checkpoint,
) -> Result<Checkpoint, AppError> {
    let mut checkpoint = checkpoint.clone();
    if let Some(settings) = settings {
        let page = settings
            .resolve_page(
                definition,
                &snapshot.relevant,
                checkpoint.page_id.as_deref(),
                checkpoint.group_id.as_deref(),
            )
            .map_err(|issue| invalid().with_issues(vec![issue]))?;
        checkpoint.page_id = Some(page.id);
        checkpoint.group_id = page.group_id;
        return Ok(checkpoint);
    }
    if checkpoint.page_id.is_some() {
        return Err(invalid());
    }
    if checkpoint
        .group_id
        .as_ref()
        .is_none_or(|id| !snapshot.groups.contains(id))
    {
        checkpoint.group_id = snapshot.groups.first().cloned();
    }
    Ok(checkpoint)
}

pub(super) fn cursor(checkpoint: &Checkpoint) -> PageCursor {
    PageCursor {
        page_id: checkpoint.page_id.clone(),
        group_id: checkpoint.group_id.clone(),
    }
}

pub(super) fn initialization_error(issues: Vec<rustsurvey_domain::ValidationIssue>) -> AppError {
    if issues
        .iter()
        .any(|issue| issue.path.starts_with("checkpoint."))
    {
        return invalid().with_issues(issues);
    }
    super::invalid(issues)
}

fn invalid() -> AppError {
    AppError::new(
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid_checkpoint",
        "Page incompatible avec le format de cette campagne.",
    )
}
