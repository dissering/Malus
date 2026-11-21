use adw::{HeaderBar, ViewStack, ViewSwitcher, ViewSwitcherPolicy};

pub fn bar(stack: &ViewStack) -> HeaderBar {
    let switcher = ViewSwitcher::builder()
        .stack(stack)
        .policy(ViewSwitcherPolicy::Narrow)
        .build();

    let header = HeaderBar::new();
    header.set_title_widget(Some(&switcher));

    header
}
