//! Luokkavalinnan esitysmalli. Ei muuta kilpailua tai tulostuksen kohteita.
use super::{ClassSelectionLogic, DrawClassOption};
use slint::{Model, ModelExt, ModelRc, VecModel};

fn indices(classes: ModelRc<DrawClassOption>, query: &str, selected_only: bool) -> Vec<i32> {
    classes.model_tracker().track_row_count_changes();
    let query = query.trim().to_lowercase();
    (0..classes.row_count())
        .filter(|&index| {
            if query.is_empty() && !selected_only {
                return true;
            }
            classes.row_data_tracked(index).is_some_and(|row| {
                (!selected_only || row.selected)
                    && (row.label.to_lowercase().contains(&query)
                        || row.code.to_lowercase().contains(&query))
            })
        })
        .filter_map(|index| i32::try_from(index).ok())
        .collect()
}

pub(super) fn register(logic: ClassSelectionLogic<'_>) {
    logic.on_export_options(|classes| {
        classes.model_tracker().track_row_count_changes();
        ModelRc::new(VecModel::from(
            (0..classes.row_count())
                .filter_map(|i| classes.row_data_tracked(i))
                .map(|row| DrawClassOption {
                    code: row.code,
                    label: row.label,
                    selected: row.selected,
                })
                .collect::<Vec<_>>(),
        ))
    });
    logic.on_indices(|classes, query, selected_only| {
        ModelRc::new(VecModel::from(indices(classes, &query, selected_only)))
    });
    logic.on_count(|classes| {
        classes.model_tracker().track_row_count_changes();
        (0..classes.row_count())
            .filter(|&index| {
                classes
                    .row_data_tracked(index)
                    .is_some_and(|row| row.selected)
            })
            .count() as i32
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filtering_preserves_source_indices_and_current_selection() {
        let rows = std::rc::Rc::new(VecModel::from(vec![
            DrawClassOption {
                code: "A:C".into(),
                label: "C — Perusluokka".into(),
                selected: true,
            },
            DrawClassOption {
                code: "B:BC".into(),
                label: "Beginners Cup".into(),
                selected: false,
            },
            DrawClassOption {
                code: "A:SM-M".into(),
                label: "SM-M".into(),
                selected: true,
            },
        ]));
        let model: ModelRc<_> = rows.clone().into();
        assert_eq!(indices(model.clone(), "", false), vec![0, 1, 2]);
        assert_eq!(indices(model.clone(), " cup ", false), vec![1]);
        assert_eq!(indices(model.clone(), "sm-m", false), vec![2]);
        assert_eq!(indices(model.clone(), "", true), vec![0, 2]);
        assert!(indices(model.clone(), "cup", true).is_empty());
        let mut row = rows.row_data(1).unwrap();
        row.selected = true;
        rows.set_row_data(1, row);
        assert_eq!(indices(model.clone(), "cup", true), vec![1]);
        assert!(indices(model, "puuttuu", false).is_empty());
    }
}
