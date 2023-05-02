use adw::prelude::*;

use crate::config;
use crate::config::RELATIVE;
use crate::ui;
use crate::ui::builder::DialogExcludePattern;
use crate::ui::prelude::*;

pub fn show(edit_exclude: Option<config::Exclude<{ RELATIVE }>>) {
    let ui = DialogExcludePattern::new();
    let dialog = ui.dialog();

    if let Some(config::Exclude::Pattern(ref pattern)) = edit_exclude {
        ui.add().set_label(&gettext("Save"));
        ui.pattern().set_text(&pattern.pattern().to_string_lossy());

        match pattern {
            config::Pattern::Fnmatch(_) => ui.pattern_type().set_selected(0),
            config::Pattern::RegularExpression(_) => ui.pattern_type().set_selected(1),
            _ => {}
        }
    }

    dialog.set_transient_for(Some(&main_ui().window()));
    ui.add().connect_clicked(
        clone!(@weak ui => move |_| Handler::run(clicked(ui, edit_exclude.clone()))),
    );

    // ensure lifetime until window closes
    let mutex = std::sync::Mutex::new(Some(ui.clone()));
    ui.dialog().connect_close_request(move |_| {
        *mutex.lock().unwrap() = None;
        gtk::Inhibit(false)
    });

    dialog.show();
}

async fn clicked(
    ui: DialogExcludePattern,
    edit_exclude: Option<config::Exclude<{ RELATIVE }>>,
) -> Result<()> {
    let selected = ui.pattern_type().selected();
    let pattern = ui.pattern().text();

    let exclude = config::Exclude::from_pattern(match selected {
        // FIXME: Manual construction
        0 => Ok(config::Pattern::fnmatch(pattern.as_str())),
        1 => config::Pattern::from_regular_expression(pattern)
            .err_to_msg(gettext("Invalid Regular Expression")),
        // Not translated because this should not happen
        _ => Err(Message::short("No valid pattern type selected").into()),
    }?);

    BACKUP_CONFIG.update_result(move |config| {
        let active = config.active_mut()?;

        if let Some(ref edit_exclude) = edit_exclude {
            active.exclude.remove(edit_exclude);
        }

        active.exclude.insert(exclude.clone());

        Ok(())
    })?;

    ui.dialog().destroy();
    ui::page_backup::refresh()?;

    Ok(())
}
