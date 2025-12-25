use crate::language::LOC;
use crate::util::to_wide;

use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::UI::Controls::{
    TASKDIALOG_BUTTON, TASKDIALOG_COMMON_BUTTON_FLAGS, TASKDIALOGCONFIG, TASKDIALOGCONFIG_0,
    TASKDIALOGCONFIG_1, TDF_ALLOW_DIALOG_CANCELLATION, TaskDialogIndirect,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::PCWSTR;

pub fn show_about_dialog(hwnd: isize) {
    let title = format!("{} CapsGlow", LOC.about);
    let app_name = "CapsGlow";
    let version = env!("CARGO_PKG_VERSION");
    let author = "iKineticate";
    let website = "https://github.com/iKineticate/CapsGlow";
    let latest_website = "https://github.com/iKineticate/CapsGlow/releases/latest";

    std::thread::spawn(move || {
        unsafe {
            let title_w = to_wide(&title);

            let message = format!(
                "   {}: {version}\n   {}: {author}\n   {}: {website}",
                LOC.version, LOC.author, LOC.website
            );
            let content_w = to_wide(&message);

            let main_instruction_w = to_wide(app_name);
            let github_button_text = to_wide(LOC.open_github);
            let view_updates_button_text = to_wide(LOC.view_updates);
            let cancel_button_text = to_wide(LOC.cancel);

            let buttons = [
                TASKDIALOG_BUTTON {
                    nButtonID: 100,
                    pszButtonText: PCWSTR(github_button_text.as_ptr()),
                },
                TASKDIALOG_BUTTON {
                    nButtonID: 200,
                    pszButtonText: PCWSTR(view_updates_button_text.as_ptr()),
                },
                TASKDIALOG_BUTTON {
                    nButtonID: 300,
                    pszButtonText: PCWSTR(cancel_button_text.as_ptr()),
                },
            ];

            let config = TASKDIALOGCONFIG {
                cbSize: std::mem::size_of::<TASKDIALOGCONFIG>() as u32,
                hwndParent: HWND(hwnd as *mut std::ffi::c_void),
                dwFlags: TDF_ALLOW_DIALOG_CANCELLATION,
                pszWindowTitle: PCWSTR(title_w.as_ptr()),
                pszMainInstruction: PCWSTR(main_instruction_w.as_ptr()),
                pszContent: PCWSTR(content_w.as_ptr()),
                Anonymous1: TASKDIALOGCONFIG_0 {
                    pszMainIcon: PCWSTR::null(),
                },
                Anonymous2: TASKDIALOGCONFIG_1 {
                    pszFooterIcon: PCWSTR::null(),
                },
                dwCommonButtons: TASKDIALOG_COMMON_BUTTON_FLAGS(0),
                pButtons: buttons.as_ptr(),
                cButtons: buttons.len() as u32,
                nDefaultButton: 300,
                pRadioButtons: std::ptr::null(),
                cRadioButtons: 0,
                cxWidth: 250,
                hInstance: HINSTANCE(std::ptr::null_mut()),
                pfCallback: None,
                lpCallbackData: 0,
                nDefaultRadioButton: 0,
                pszCollapsedControlText: PCWSTR::null(),
                pszExpandedControlText: PCWSTR::null(),
                pszExpandedInformation: PCWSTR::null(),
                pszVerificationText: PCWSTR::null(),
                pszFooter: PCWSTR::null(),
            };

            let mut pn_button: i32 = 0;

            let result = TaskDialogIndirect(&config, Some(&mut pn_button), None, None);

            if result.is_err() {
                return;
            }

            match pn_button {
                100 => {
                    // 打开 GitHub
                    let url = to_wide(website);

                    ShellExecuteW(
                        None,
                        PCWSTR(to_wide("open").as_ptr()),
                        PCWSTR(url.as_ptr()),
                        PCWSTR::null(),
                        PCWSTR::null(),
                        SW_SHOWNORMAL,
                    );
                }
                200 => {
                    // 打开 GitHub Release
                    let url = to_wide(latest_website);

                    ShellExecuteW(
                        None,
                        PCWSTR(to_wide("open").as_ptr()),
                        PCWSTR(url.as_ptr()),
                        PCWSTR::null(),
                        PCWSTR::null(),
                        SW_SHOWNORMAL,
                    );
                }
                _ => (),
            }
        }
    });
}
