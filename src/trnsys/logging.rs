use crate::TYPE_NUMBER;
use crate::trnsys::{Severity, get_current_unit, get_simulation_time, log_message, messages, simulation_has_error};
use std::backtrace;
use std::fmt::{Debug, Formatter, Pointer};
use std::fs::OpenOptions;
use std::io::{Cursor, Write};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber, error};
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::fmt::{FormatEvent, FormatFields, format, time};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{
    Layer,
    fmt::{self, time::OffsetTime, writer::BoxMakeWriter},
    layer::SubscriberExt,
    registry::Registry,
};


const TRNSYS_LOG_LEVEL: Level = Level::INFO;


/// Custom function to handle trnsys logging.
///
/// # Arguments
///
/// * `level` - The level of the event.
/// * `name` - The name of the event.
/// * `message` - The message of the event.
pub fn log_in_trnsys(level: Level, error_code: Option<i32>, message: &str) {
    // Perform necessary actions here, such as sending notifications, writing to a dedicated log, raising alarms, etc.
    let trnsys_severity = match level {
        Level::ERROR => Severity::Fatal,
        Level::WARN => Severity::Warning,
        _ => Severity::Notice,
    };
    let error_code = error_code.unwrap_or(-1);

    log_message(trnsys_severity, error_code, message);
}

struct MessageCollector {
    message: Option<String>,
    fields: Vec<(String, String)>,
}

impl MessageCollector {
    fn new() -> Self {
        MessageCollector {
            message: None,
            fields: Vec::new(),
        }
    }

    fn get_formatted_message(&self) -> Option<String> {
        if let Some(message) = &self.message {
            let mut formatted_message = message.clone();
            for (name, value) in &self.fields {
                formatted_message = formatted_message.replace(&format!("{{{}}}", name), value);
            }
            Some(formatted_message)
        } else {
            None
        }
    }
}

impl Visit for MessageCollector {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let val_str = format!("{:?}", value);
        if field.name() == "message" {
            // message字段通常是一个静态字符串，如 "a: {a}"
            self.message = Some(val_str.trim_matches('"').to_string());
        } else {
            // 其他字段存储参数值
            self.fields.push((field.name().to_string(), val_str));
        }
    }
}

/// Custom Layer to intercept high-priority events.
struct TrnSysLogLayer {
    threshold: Level,
    formatter: format::Format,
}

impl TrnSysLogLayer {
    pub fn new(threshold: Level) -> Self {
        TrnSysLogLayer {
            threshold,
            formatter: format::Format::default()
                .with_line_number(true)
                .with_file(true)
                .with_timer(time())
                .with_level(true)
                .with_target(true),
        }
    }
}

impl<S: Subscriber> Layer<S> for TrnSysLogLayer {
    /// Intercepts events and calls the custom function if the event level is above the threshold.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to be processed.
    /// * `_ctx` - The context of the subscriber.
    fn on_event(&self, event: &Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let metadata = event.metadata();
        if metadata.level() <= &self.threshold {
            let mut str = String::new();
            let mut writer = tracing_subscriber::fmt::format::Writer::new(&mut str);

            let mut collector = MessageCollector::new();
            event.record(&mut collector);
            let formatted_msg = collector.get_formatted_message();
            if let Some(msg) = formatted_msg {
                write!(
                    writer,
                    "{} {} {}: {}",
                    metadata.level(),
                    metadata.target(),
                    metadata.name(),
                    msg
                )
                .unwrap();
            }

            log_in_trnsys(metadata.level().clone(), None, &str);
        }
    }
}

/// Returns the default log file name under the current directory.
pub fn get_default_log_file() -> String {

    let file_name = format!(
        "type_{}_{}.log",
        TYPE_NUMBER,
        get_current_unit()
    );
    let cwd = std::env::current_dir().unwrap_or(std::env::temp_dir());
    
    cwd.join(file_name)
        .to_str()
        .expect("Failed to get log file name")
        .to_string()

}

struct UnitNoFmt<F>(F);

impl<S, N, F> FormatEvent<S, N> for UnitNoFmt<F>
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
    N: for<'writer> FormatFields<'writer> + 'static,
    F: FormatEvent<S, N>,
{
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        write!(writer, "[Unit {}] [T={}]", get_current_unit(), get_simulation_time())?;

        self.0.format_event(ctx, writer.by_ref(), event)?;

        Ok(())
    }
}

static LOGFILE_PATH: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));

pub fn is_tracing_initialized() -> bool {
    LOGFILE_PATH.try_lock().map(|l|l.is_some()).unwrap_or(true)
}

/// Initializes tracing with custom layers and settings.
///
/// # Arguments
///
/// * `file_name` - The name of the log file to write to.
pub fn init_tracing(file_name: Option<String>) {
    let file_name = file_name.unwrap_or(get_default_log_file());

    // Store the log file path for later use
    let mut log_file_path = LOGFILE_PATH.lock().unwrap();
    *log_file_path = Some(file_name.clone());

    // Open (or create) the log file
    let log_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(file_name)
        .expect("Failed to open log file");

    // Wrap the writer with a Mutex to ensure thread-safe writing
    let file_writer = BoxMakeWriter::new(log_file);

    let local_time = OffsetTime::local_rfc_3339().expect("Failed to get local time offset");

    // Set up the filter (can be controlled via the RUST_LOG environment variable)
    #[cfg(debug_assertions)]
    let filter = EnvFilter::from_env("TRNSYS_ODBC_RS").add_directive("debug".parse().unwrap());
    #[cfg(not(debug_assertions))]
    let filter = EnvFilter::from_env("TRNSYS_ODBC_RS").add_directive("info".parse().unwrap());

    // Formatting Layer: output to both file and stdout
    let fmt_layer = fmt::layer()
        .with_writer(file_writer.and(std::io::stdout))
        .with_timer(local_time)
        .with_line_number(true)
        .with_file(true)
        .with_ansi(false)
        .event_format(UnitNoFmt(fmt::format()));

    let trnsys_log_layer = TrnSysLogLayer::new(TRNSYS_LOG_LEVEL);

    // Combine layers
    let subscriber = Registry::default()
        .with(filter)
        .with(trnsys_log_layer)
        .with(fmt_layer);

    // Global initialization
    tracing::subscriber::set_global_default(subscriber)
        .expect("Unable to set global tracing subscriber");

    // panic hook
    std::panic::set_hook(Box::new(|panic_info| {
        error!("TrnSys Type Panicked: {:#}", panic_info);

    }));
}

