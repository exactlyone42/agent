use crate::error::XtaskError;
use crate::markdown_html::markdown_html;
use crate::plan_page::PlanPage;
use crate::section_between::section_between;

pub fn fault_page(evaluation: &str) -> Result<PlanPage, XtaskError> {
    let section = section_between(evaluation, "## 故障清單與可觀測的結果", "## 報表與回歸")?;
    Ok(PlanPage::new(
        "faults",
        "故障注入覆蓋｜正常、故障、失敗對照都要有",
        markdown_html(section),
    ))
}
