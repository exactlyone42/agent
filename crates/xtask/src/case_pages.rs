use crate::case_card::CaseCard;
use crate::error::XtaskError;
use crate::plan_page::PlanPage;

const NOTE: &str = "<p class=\"note\">共用案例只在 Linux 實機執行，Windows／macOS 不做實機驗收。具名執行者／真人驗收者在實跑前填入；不得以角色名稱或 AI 模擬簽收。安全與假成功反例未過就不能放行。</p>";

pub fn case_pages(evaluation: &str) -> Result<Vec<PlanPage>, XtaskError> {
    let cards = CaseCard::parse_all(evaluation)?;
    cards
        .chunks(4)
        .enumerate()
        .map(|(index, chunk)| {
            let articles = chunk
                .iter()
                .map(CaseCard::html)
                .collect::<Result<String, _>>()?;
            let first = chunk.first().map(|c| c.id.as_str()).unwrap_or_default();
            let last = chunk.last().map(|c| c.id.as_str()).unwrap_or_default();
            Ok(PlanPage::new(
                &format!("cases-{}", index + 1),
                &format!("驗收操作卡｜{first}–{last}"),
                format!("<div class=\"cards\">{articles}</div>{NOTE}"),
            ))
        })
        .collect()
}
