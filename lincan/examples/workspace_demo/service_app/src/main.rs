use core_types::{display_name, Role, User, UserMarker};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct UserReport {
    label: String,
    is_admin: bool,
}

fn build_report(user: &User) -> UserReport {
    let _marker = UserMarker;
    let report_line = user.report_line();
    let display = display_name(user);
    UserReport {
        label: format!("{report_line} | {display}"),
        is_admin: user.is_admin() && matches!(user.role, Role::Admin),
    }
}

fn main() {
    let user = User {
        id: 1,
        name: String::from("alice"),
        role: Role::Admin,
    };
    let report = build_report(&user);
    println!("{} {}", report.label, report.is_admin);
}
