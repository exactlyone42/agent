use crate::error::XtaskError;
use crate::plan_page::PlanPage;
use crate::section_between::section_between;
use crate::table_html::table_html;
use crate::table_rows::table_rows;

const NOTE: &str = "<p class=\"note\">每個部件另須有：輸入輸出版本、權限、資料擁有者、錯誤與取消、重試／副作用、資源、觀測、遷移／回退、上下游整合。相關驗收卡見前頁需求群對照。</p>";

pub fn contract_pages(architecture: &str) -> Result<Vec<PlanPage>, XtaskError> {
    let section = section_between(architecture, "## 部件責任與替換驗收", "## 整體完整性檢查")?;
    let mut rows = table_rows(section);
    if rows.len() < 2 {
        return Err(XtaskError::Report("部件契約表沒有資料列"));
    }
    let headers = rows.remove(0);
    Ok(rows
        .chunks(7)
        .enumerate()
        .map(|(index, chunk)| {
            let number = index + 1;
            let body = format!("{}{NOTE}", table_html(&headers, chunk, "contract"));
            PlanPage::new(
                &format!("contracts-{number}"),
                &format!("部件契約 {number}｜責任、替換與故障驗收"),
                body,
            )
        })
        .collect())
}
