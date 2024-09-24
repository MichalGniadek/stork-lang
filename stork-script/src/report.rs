use ariadne::Color;
use std::ops::Range;

use crate::module_index::ModuleID;

pub type Span = (ModuleID, Range<usize>);
pub type Label = ariadne::Label<Span>;
pub type ReportBuilder = ariadne::ReportBuilder<'static, Span>;
pub type Report = ariadne::Report<'static, Span>;
pub type Result<T, E = Box<Report>> = std::result::Result<T, E>;
pub type ReportKind = ariadne::ReportKind<'static>;
pub const INTERNAL_REPORT_KIND: ReportKind = ReportKind::Custom("internal", Color::BrightMagenta);
