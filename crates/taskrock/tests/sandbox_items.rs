//! The project items of the plan on GitHub.

use taskrock::gh::RealGh;
use taskrock::github::items::project_items;
use taskrock::link::ProjectLink;

#[test]
#[ignore = "talks to GitHub"]
fn reads_the_items_of_project_three() {
    let project = ProjectLink {
        owner: "tailrocks".to_owned(),
        number: 3,
    };
    let items = project_items(&RealGh::default(), &project).expect("GitHub answers");
    assert!(items.len() >= 23, "at least 23 items, got {}", items.len());
    for item in &items {
        if item.kind.as_deref() == Some("Task") {
            assert!(item.parent.is_some(), "task {} has a parent", item.title);
        }
    }
}
