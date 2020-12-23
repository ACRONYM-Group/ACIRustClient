/// Error type for errors originating from the ACI Client
#[derive(Debug, Clone)]
pub struct ACIClientError
{
    data: String
}

impl ACIClientError
{
    pub fn new(data: String) -> Self
    {
        Self
        {
            data
        }
    }
}

impl std::fmt::Display for ACIClientError
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
    {
        write!(f, "{}", self.data)
    }
}

/// Error type for errors returned from the ACI server
#[derive(Debug, Clone)]
pub struct ACIServerError
{
    data: String
}

impl ACIServerError
{
    pub fn new(data: String) -> Self
    {
        Self
        {
            data
        }
    }
}

/// Common Error Type
#[derive(Debug)]
pub enum ACIError
{
    ClientError(ACIClientError),
    ServerError(ACIServerError),
}

impl std::fmt::Display for ACIServerError
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
    {
        write!(f, "{}", self.data)
    }
}
