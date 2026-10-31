use crate::case_pages::case_pages;
use crate::contract_pages::contract_pages;
use crate::error::XtaskError;
use crate::fault_page::fault_page;
use crate::markdown_html::markdown_html;
use crate::plan_page::PlanPage;
use std::path::Path;

const NAV: &str = "<nav><b>Agent Platform</b><a href=\"#start\">總覽</a><a href=\"#deploy\">架構</a><a href=\"#rag\">RAG</a><a href=\"#platform\">平台範圍</a><a href=\"#accept\">驗收</a><a href=\"#cases-1\">操作卡</a><a href=\"#canonical\">主計畫全文</a><button onclick=\"window.print()\">列印</button></nav>";

fn read(path: &Path) -> Result<String, XtaskError> {
    Ok(std::fs::read_to_string(path)?)
}

pub fn plan_document(root: &Path) -> Result<String, XtaskError> {
    let docs = root.join("docs");
    let evaluation = read(&docs.join("evaluation.md"))?;
    let architecture = read(&docs.join("architecture.md"))?;
    let canonical = read(&root.join("PLAN.md"))?;
    let css = read(&docs.join("plan-pages/style.css"))?;
    let mut pages = PlanPage::read_all(&docs.join("plan-pages"))?;
    pages.extend(contract_pages(&architecture)?);
    pages.extend(case_pages(&evaluation)?);
    pages.push(fault_page(&evaluation)?);
    let sections: String = pages
        .iter()
        .enumerate()
        .map(|(index, page)| page.section(index + 1))
        .collect();
    let appendix = format!(
        "<section class=\"appendix\" id=\"canonical\"><h1>附錄｜唯一主計畫全文</h1><p>以下由根目錄 PLAN.md 同次產生，與前面圖形及驗收卡共同閱讀。</p>{}</section>",
        markdown_html(&canonical)
    );
    Ok(format!(
        "<!doctype html><html lang=\"zh-Hant\"><head><meta charset=\"utf-8\"><title>Agent Platform｜架構與完整驗收</title><style>{css}</style></head><body>{NAV}{sections}{appendix}</body></html>"
    ))
}
