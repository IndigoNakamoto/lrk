use brk_error::Result;
use brk_types::Height;

pub trait AnyStore: Send + Sync {
    fn height(&self) -> Option<Height>;
    fn export_meta(&mut self, height: Height) -> Result<()>;
    fn commit(&mut self, height: Height) -> Result<()>;
}
