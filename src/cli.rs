use std::borrow::Cow;
use std::fmt::Display;
use std::time::Duration;

use console::Alignment;
use indicatif::{ProgressBar, ProgressStyle};
use lazy_static::lazy_static;

pub const PRINT_PADDING: usize = 9;
const SPINNER_INTERVAL: Duration = Duration::from_millis(100);

lazy_static! {
    pub static ref SPINNER_STYLE: ProgressStyle =
        ProgressStyle::with_template("{spinner:.magenta} {msg}")
            .expect("is valid schema")
            .tick_strings(&["/", "-", "\\", "|"]);
    pub static ref PROGRESS_STYLE: ProgressStyle = ProgressStyle::with_template(
        format!(
            "{{msg:.cyan}} {{wide_bar:.magenta/.234}} \
             {{human_pos:.magenta}}{slash}{{human_len:.magenta}} {{prefix:.cyan}}",
            slash = console::style("/").magenta()
        )
        .as_str()
    )
    .expect("is valid schema")
    .progress_chars("━ ━");
}

pub fn progress_message<M, S>(
    progress: &ProgressBar,
    keyword: impl Display,
    color: console::Color,
    msg: M
) where
    M: Into<Option<S>>,
    S: Display
{
    let keyword = console::style(keyword).fg(color);
    let keyword = keyword.to_string();
    let keyword = console::pad_str(keyword.as_str(), PRINT_PADDING, Alignment::Right, None);

    let msg = msg.into();
    let msg: &dyn Display = match msg.as_ref() {
        Some(m) => m,
        None => &""
    };

    progress.println(format!("{keyword} {msg}"))
}

pub struct ProgressBarGuard {
    pub progress_bar: ProgressBar,
    finish_message: Option<String>
}

impl ProgressBarGuard {
    pub fn new(progress_bar: ProgressBar, finish_message: Option<String>) -> Self {
        ProgressBarGuard {
            progress_bar,
            finish_message
        }
    }

    pub fn new_wait_spinner(msg: impl Into<Cow<'static, str>>) -> Self {
        let spinner =
            ProgressBar::new_spinner().with_message(msg).with_style(SPINNER_STYLE.clone());
        spinner.enable_steady_tick(SPINNER_INTERVAL);
        Self::new(spinner, None)
    }
}

impl Drop for ProgressBarGuard {
    fn drop(&mut self) {
        self.progress_bar.disable_steady_tick();
        match self.finish_message.as_deref() {
            Some(msg) => self.progress_bar.finish_with_message(msg.to_string()),
            None => self.progress_bar.finish_and_clear()
        };
    }
}
