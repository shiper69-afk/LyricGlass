#[cfg(windows)]
pub fn block_async<T>(operation: windows::Foundation::IAsyncOperation<T>) -> windows::core::Result<T>
where
    T: windows::core::RuntimeType,
{
    use windows::Foundation::AsyncStatus;
    for _ in 0..800 {
        match operation.Status()? {
            AsyncStatus::Started => {
                std::thread::sleep(std::time::Duration::from_millis(15));
            }
            _ => break,
        }
    }
    operation.GetResults()
}