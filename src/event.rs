pub struct ACIEvent
{
    pub source: String,
    pub destination: String,
    data: serde_json::Value,
    pub event_id: serde_json::Value
}

impl ACIEvent
{
    /// Create an ACIEvent from raw values
    pub fn create(source: String, destination: String, data: serde_json::Value, event_id: serde_json::Value) -> Self
    {
        Self
        {
            source,
            destination,
            data,
            event_id
        }
    }

    /// Consume the event and extract the data
    pub fn consume(self) -> serde_json::Value
    {
        self.data
    }
}