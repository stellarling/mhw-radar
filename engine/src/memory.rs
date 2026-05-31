//! 内存读取工具：坐标结构体、进程基址获取、指针链解析
use process_memory::{CopyAddress, ProcessHandle};

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector3 {}

/// 通过 PID 获取进程模块基址
pub fn get_module_base(pid: u32) -> Option<u64> {
    unsafe {
        use winapi::shared::minwindef::HMODULE;
        use winapi::um::handleapi::CloseHandle;
        use winapi::um::processthreadsapi::OpenProcess;
        use winapi::um::psapi::EnumProcessModules;
        use winapi::um::winnt::{PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};

        let handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid);
        if handle.is_null() {
            return None;
        }

        let mut module_handle: HMODULE = std::ptr::null_mut();
        let mut cb_needed = 0;

        let success = EnumProcessModules(
            handle,
            &mut module_handle,
            std::mem::size_of::<HMODULE>() as u32,
            &mut cb_needed,
        );
        CloseHandle(handle);

        if success != 0 {
            Some(module_handle as u64)
        } else {
            None
        }
    }
}

/// 从指定地址读取 T 类型数据
pub fn read_memory<T: Copy>(handle: ProcessHandle, addr: u64) -> Option<T> {
    if addr == 0 {
        return None;
    }
    let mut buffer = [0u8; 64];
    let size = std::mem::size_of::<T>();
    if handle.copy_address(addr as usize, &mut buffer[..size]).is_ok() {
        unsafe { Some(std::ptr::read_unaligned(buffer.as_ptr() as *const T)) }
    } else {
        None
    }
}

/// 解析多层指针链：逐级解引用 + 偏移
pub fn resolve_pointer(handle: ProcessHandle, base: u64, offsets: &[u64]) -> Option<u64> {
    if base == 0 {
        return None;
    }
    let mut current_addr = base;
    for &offset in offsets {
        match read_memory::<u64>(handle, current_addr) {
            Some(0) => return None,
            Some(ptr) => current_addr = ptr + offset,
            None => return None,
        }
    }
    Some(current_addr)
}

/// 逐级解引用指针链：每步先加偏移再读取下级指针
pub fn read_ptr_chain(handle: ProcessHandle, mut addr: u64, offsets: &[u64]) -> Option<u64> {
    for &offset in offsets {
        let new_addr = addr + offset;
        match read_memory::<u64>(handle, new_addr) {
            Some(0) | None => return None,
            Some(next) => addr = next,
        }
    }
    Some(addr)
}

/// 从进程内存批量读取 N 个 u64 值（用于读取怪物组件数组等固定大小数组）
pub fn read_array_u64(handle: ProcessHandle, addr: u64, count: usize) -> Option<Vec<u64>> {
    if addr == 0 || count == 0 {
        return None;
    }
    let size = count * 8;
    let mut buffer = vec![0u8; size];
    if handle.copy_address(addr as usize, &mut buffer).is_ok() {
        let mut result = Vec::with_capacity(count);
        for i in 0..count {
            let offset = i * 8;
            let val = unsafe {
                std::ptr::read_unaligned(buffer[offset..offset + 8].as_ptr() as *const u64)
            };
            result.push(val);
        }
        Some(result)
    } else {
        None
    }
}

/// 从游戏进程读取指定地址的字符串（最多 max_len 字节，遇 null 截断）
pub fn read_string(handle: ProcessHandle, addr: u64, max_len: usize) -> Option<String> {
    if addr == 0 {
        return None;
    }
    let mut buffer = vec![0u8; max_len];
    if handle.copy_address(addr as usize, &mut buffer).is_err() {
        return None;
    }
    let end = buffer.iter().position(|&b| b == 0).unwrap_or(max_len);
    let s = String::from_utf8_lossy(&buffer[..end]).to_string();
    if s.is_empty() { None } else { Some(s) }
}
