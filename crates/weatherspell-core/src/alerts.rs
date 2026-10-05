// The Alerts section's fixed lines. The alerts themselves (the NWS and
// Environment Canada, the details, the announcements) come with the
// alerts step of the port; until then every forecast says alerts are not
// available, as the C# app does for a region without them.

pub const HEADING: &str = "Alerts";
pub const NOT_AVAILABLE_LINE: &str = "Alerts are not available for this region.";
pub const NONE_LINE: &str = "No alerts in effect.";
