pub fn table_html(headers: &[String], rows: &[Vec<String>], class: &str) -> String {
    let head: String = headers
        .iter()
        .map(|cell| format!("<th>{cell}</th>"))
        .collect();
    let body: String = rows
        .iter()
        .map(|row| {
            let cells: String = row.iter().map(|cell| format!("<td>{cell}</td>")).collect();
            format!("<tr>{cells}</tr>")
        })
        .collect();
    format!("<table class=\"{class}\"><thead><tr>{head}</tr></thead><tbody>{body}</tbody></table>")
}
