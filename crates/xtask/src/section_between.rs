use crate::error::XtaskError;

pub fn section_between<'a>(
    text: &'a str,
    start: &'static str,
    end: &'static str,
) -> Result<&'a str, XtaskError> {
    let (_, after) = text
        .split_once(start)
        .ok_or(XtaskError::Report("文件缺少起始標題"))?;
    let (section, _) = after
        .split_once(end)
        .ok_or(XtaskError::Report("文件缺少結束標題"))?;
    Ok(section)
}
