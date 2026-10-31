use agent_platform_xtask::{XtaskError, plan_document};
use std::fs;
use std::path::Path;

const CARD: &str = "### A01｜安裝\n\n- 階段：P03a\n- 前置：甲\n- 操作：乙\n- 通過：丙\n- 失敗對照：丁\n- 證據：戊\n- AI／真人：己\n";

fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn fake_root(card: &str) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "PLAN.md",
        "# 計畫\n\n[本地](docs/x.md) [外部](https://example.com) [錨](#a)\n",
    );
    write(root.path(), "docs/plan-pages/style.css", "body{}");
    write(
        root.path(),
        "docs/plan-pages/01-start.html",
        "標題一\n<p>內容一</p>",
    );
    write(
        root.path(),
        "docs/architecture.md",
        "## 部件責任與替換驗收\n\n| 部件 | 責任 |\n| --- | --- |\n| 核心 | 迴圈 |\n\n## 整體完整性檢查\n",
    );
    let evaluation = format!(
        "## 故障清單與可觀測的結果\n\n| 故障 | 結果 |\n| --- | --- |\n| F01 | 停止 |\n\n## 報表與回歸\n\n{card}"
    );
    write(root.path(), "docs/evaluation.md", &evaluation);
    root
}

#[test]
fn builds_pages_in_order_and_strips_local_links() {
    let root = fake_root(CARD);
    let html = plan_document(root.path()).unwrap();
    let order = [
        "id=\"start\"",
        "id=\"contracts-1\"",
        "id=\"cases-1\"",
        "id=\"faults\"",
        "id=\"canonical\"",
    ];
    let positions: Vec<usize> = order.iter().map(|key| html.find(key).unwrap()).collect();
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(html.contains("<td>迴圈</td>"));
    assert!(html.contains("A01｜安裝"));
    assert!(html.contains("href=\"https://example.com\""));
    assert!(html.contains("href=\"#a\""));
    assert!(!html.contains("docs/x.md"));
    assert!(html.contains("本地"));
}

#[test]
fn card_missing_a_field_is_rejected() {
    let root = fake_root(&CARD.replace("- 證據：戊\n", ""));
    let result = plan_document(root.path());
    assert!(matches!(result, Err(XtaskError::Report(_))));
}

#[test]
fn no_cards_is_rejected() {
    let root = fake_root("沒有任何驗收卡\n");
    assert!(matches!(
        plan_document(root.path()),
        Err(XtaskError::Report(_))
    ));
}
