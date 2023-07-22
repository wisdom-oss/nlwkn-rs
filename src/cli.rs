use indicatif::{ProgressBar, ProgressStyle};
use std::borrow::Cow;
use std::time::Duration;

const SPINNER_INTERVAL: Duration = Duration::from_millis(100);

pub struct ProgressBarGuard {
    pub progress_bar: ProgressBar,
    finish_message: Option<String>,
}

impl ProgressBarGuard {
    pub fn new(progress_bar: ProgressBar, finish_message: Option<String>) -> Self {
        ProgressBarGuard {
            progress_bar,
            finish_message,
        }
    }

    pub fn new_wait_spinner(msg: impl Into<Cow<'static, str>>) -> Self {
        let spinner = ProgressBar::new_spinner().with_message(msg).with_style(
            ProgressStyle::with_template("{spinner:.magenta} {msg}")
                .expect("is valid schema")
                .tick_strings(&["/", "-", "\\", "|"]),
        );
        spinner.enable_steady_tick(SPINNER_INTERVAL);
        Self::new(spinner, None)
    }
}

impl Drop for ProgressBarGuard {
    fn drop(&mut self) {
        self.progress_bar.disable_steady_tick();
        match self.finish_message.as_deref() {
            Some(msg) => self.progress_bar.finish_with_message(msg.to_string()),
            None => self.progress_bar.finish_and_clear(),
        };
    }
}
