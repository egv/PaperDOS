pub trait DisplayTransport {
    type Error;

    fn reset(&mut self) -> Result<(), Self::Error>;
    fn wait_while_busy(&mut self) -> Result<(), Self::Error>;
    fn write_command(&mut self, command: u8) -> Result<(), Self::Error>;
    fn write_data(&mut self, data: &[u8]) -> Result<(), Self::Error>;
}
