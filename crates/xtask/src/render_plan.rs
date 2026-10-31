use crate::chrome_print::chrome_print;
use crate::error::XtaskError;
use crate::plan_document::plan_document;
use std::path::Path;

pub fn render_plan(root: &Path) -> Result<i32, XtaskError> {
    let html = plan_document(root)?;
    let folder = std::env::temp_dir().join(format!("render-plan-{}", std::process::id()));
    std::fs::create_dir_all(&folder)?;
    let html_path = folder.join("PLAN.html");
    let pdf_path = folder.join("PLAN.pdf");
    std::fs::write(&html_path, html)?;
    let printed = chrome_print(&html_path, &pdf_path);
    let copied = printed.and_then(|()| Ok(std::fs::copy(&pdf_path, root.join("docs/PLAN.pdf"))?));
    std::fs::remove_dir_all(&folder)?;
    copied?;
    Ok(0)
}
