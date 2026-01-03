use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::core::*;
use windows::Win32::Foundation::{HMODULE, HWND};
use windows::Win32::Graphics::Direct2D::D2D1CreateDevice;
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::DirectComposition::*;
use windows::Win32::Graphics::Dxgi::IDXGIDevice3;
use winit::window::Window;

pub struct Dx12Surface {
    #[allow(unused)]
    device: ID3D11Device,
    pub desktop: IDCompositionDesktopDevice,
    #[allow(unused)]
    target: IDCompositionTarget,
    pub wgpu_visual: IDCompositionVisual2,
}

impl Dx12Surface {
    pub fn new(window: &Window) -> anyhow::Result<Self> {
        let device = unsafe {
            let mut device = None;
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                None,
            )
            // SAFETY: D3D11CreateDevice succeded, device is Some
            .map(|()| device.unwrap())?
        };

        let dxgi3: IDXGIDevice3 = device.cast()?;
        let device_2d = unsafe { D2D1CreateDevice(&dxgi3, None) }?;

        let desktop: IDCompositionDesktopDevice = unsafe { DCompositionCreateDevice2(&device_2d)? };

        let hwnd = window.window_handle()?;
        let RawWindowHandle::Win32(hwnd) = hwnd.as_raw() else {
            anyhow::bail!("Window handle must be win32");
        };
        let hwnd = HWND(hwnd.hwnd.get() as _);

        let target = unsafe { desktop.CreateTargetForHwnd(hwnd, true) }?;

        let root_visual = unsafe { desktop.CreateVisual() }?;
        unsafe { target.SetRoot(&root_visual) }?;

        let wgpu_visual = unsafe { desktop.CreateVisual() }?;
        unsafe { root_visual.AddVisual(&wgpu_visual, false, None) }?;

        Ok(Self {
            desktop,
            device,
            target,
            wgpu_visual,
        })
    }
}
