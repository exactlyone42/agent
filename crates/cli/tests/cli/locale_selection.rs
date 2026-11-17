use crate::case::{ENGLISH, LANGUAGE_ENV, TRADITIONAL_CHINESE};
use crate::run_each::run_each;
use crate::usage_with::usage_with;

const ENGLISH_USAGE: &str = "Usage:";
const CHINESE_USAGE: &str = "用法：";

#[test]
fn locale_follows_the_documented_variable_order() {
    let cases: [(&[(&str, &str)], &str); 13] = [
        (&[], ENGLISH_USAGE),
        (&[(LANGUAGE_ENV, ENGLISH)], ENGLISH_USAGE),
        (&[(LANGUAGE_ENV, TRADITIONAL_CHINESE)], CHINESE_USAGE),
        (&[(LANGUAGE_ENV, "zh_TW.UTF-8")], CHINESE_USAGE),
        (&[("LANG", "zh_Hant_TW")], CHINESE_USAGE),
        (&[("LANG", "zh_TW.UTF-8")], CHINESE_USAGE),
        (
            &[("LC_ALL", "zh_TW.UTF-8"), ("LANG", "en_US.UTF-8")],
            CHINESE_USAGE,
        ),
        (
            &[("LC_ALL", "en_US.UTF-8"), ("LANG", "zh_TW.UTF-8")],
            ENGLISH_USAGE,
        ),
        (
            &[("LC_MESSAGES", "zh_TW.UTF-8"), ("LANG", "en_US")],
            CHINESE_USAGE,
        ),
        (
            &[(LANGUAGE_ENV, ENGLISH), ("LC_ALL", "zh_TW.UTF-8")],
            ENGLISH_USAGE,
        ),
        (
            &[(LANGUAGE_ENV, ""), ("LANG", "zh_TW.UTF-8")],
            CHINESE_USAGE,
        ),
        (&[("LANG", "zh_CN.UTF-8")], ENGLISH_USAGE),
        (&[("LANG", "C")], ENGLISH_USAGE),
    ];
    run_each(cases, |(variables, expected)| {
        let usage = usage_with(variables);
        assert!(usage.contains(expected), "{variables:?}: {usage}");
        let other = if expected == ENGLISH_USAGE {
            CHINESE_USAGE
        } else {
            ENGLISH_USAGE
        };
        assert!(!usage.contains(other), "{variables:?}: {usage}");
    });
}
