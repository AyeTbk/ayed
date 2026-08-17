use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
    path::Path,
};

use ayed_glob::Glob;

use crate::{
    command::{CommandRegistry, helpers::focused_buffer_command, options::Options},
    panels::list_picker::{ListPickerItem, ListPickerItemKind},
    state::{Diagnostic, DiagnosticKind, State},
};

pub fn register_list_picker_commands(cr: &mut CommandRegistry) {
    cr.register(
        "list-picker-confirm",
        "nodoc",
        focused_buffer_command(|_opt, ctx| {
            let idx = ctx.state.list_picker.selected_item;
            let Some(item) = ctx.state.list_picker.items.get(idx) else {
                return Ok(());
            };
            let command = &item.command;
            if command.trim() == "" {
                return Ok(());
            }

            ctx.queue.push("panel-focus editor");
            ctx.queue.push(command);

            Ok(())
        }),
    );

    cr.register(
        "list-picker-select",
        Options::new().doc("nodoc").flag("next").flag("previous"),
        |opt, ctx| {
            let next = opt.contains("next");
            let previous = opt.contains("previous");

            if next {
                ctx.state.list_picker.select_next();
            }
            if previous {
                ctx.state.list_picker.select_previous();
            }

            Ok(())
        },
    );

    register_file_picker_commands(cr);
    register_diagnostics_picker_commands(cr);
}

// == File picker stuff ==

fn register_file_picker_commands(cr: &mut CommandRegistry) {
    cr.register(
        "file-picker-filter-list",
        "nodoc",
        focused_buffer_command(|_opt, ctx| {
            let filter = ctx.buffer.line(0).unwrap_or_default();
            let filters: Vec<&str> = filter.split_ascii_whitespace().collect();
            let mut filtered_list = Vec::new();
            'raw_item: for item in &ctx.state.list_picker.raw_items {
                for filter in &filters {
                    // TODO FEAT case insensitivity
                    if !item.filter_text.contains(filter) {
                        continue 'raw_item;
                    }
                }
                filtered_list.push(item.clone());
            }

            ctx.state.list_picker.items = file_list_to_file_tree(filtered_list);
            ctx.state.list_picker.reselect();
            Ok(())
        }),
    );

    cr.register("file-picker-fill-list", "nodoc", |_opt, ctx| {
        let ignore = get_gitignore_ignores(&ctx.state.working_directory); // FIXME make this "ignore paths source" configurable.
        match file_picker_fill_list(&ctx.state, "", &ignore) {
            Ok(list) => {
                ctx.state.list_picker.raw_items = list.clone();
                ctx.state.list_picker.items = file_list_to_file_tree(list);
            }
            Err(err) => return Err(err.to_string()),
        }
        ctx.state.list_picker.reselect();
        Ok(())
    });
}

fn get_gitignore_ignores(cwd: &Path) -> Vec<Glob> {
    // FIXME also need to check .git/info/exclude
    let Ok(gitignore) = std::fs::read_to_string(cwd.join(".gitignore")) else {
        return vec![];
    };
    // NOTE In gitignore files, one can 'un-ignore' a file by prepending the
    // pattern with a '!'. This is sensitive to the order in which the
    // patterns are defined in the file (last one wins).
    // TODO support this '!' stuff
    gitignore
        .split('\n')
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .chain([".git/", ".jj/"])
        .map(|rule| {
            let pattern = if rule.starts_with('/') {
                Cow::Borrowed(rule)
            } else {
                Cow::Owned(format!("**/{rule}"))
            };
            Glob::new(pattern.as_ref())
        })
        .collect()
}

fn file_picker_fill_list(
    state: &State,
    filter: &str,
    ignore: &[Glob],
) -> std::io::Result<Vec<ListPickerItem>> {
    // FIXME The state param is only used for the working dir and the
    // denormalize_path method. Maybe denormalize could be a standalone util
    // and just pass the working_directory instead?
    let working_directory = state.working_directory.clone();
    fn aux(
        filters: &[&str],
        dir_path: &Path,
        list: &mut Vec<ListPickerItem>,
        state: &State,
        ignore: &[Glob],
    ) -> std::io::Result<()> {
        // FIXME This hardcoded limit really sucks
        if list.len() > 500 {
            return Ok(());
        }
        'entry: for maybe_entry in std::fs::read_dir(dir_path)? {
            let Ok(entry) = maybe_entry else { continue };
            let path = state.denormalize_path(&entry.path());

            // Filter out stuff that matches rules in .*ignore files.
            // TODO properly support making path 'rooted' at the level of their
            // respective repo, rather than at the level of the cwd.
            let path_is_file = entry.metadata().map(|m| m.is_file()).unwrap_or_default();
            let mut path_for_ignoring = Path::new("/").to_path_buf();
            path_for_ignoring.push(&path);
            for ignore_pattern in ignore {
                if ignore_pattern.is_match_path(&path_for_ignoring, path_is_file) {
                    continue 'entry;
                }
            }

            if entry.file_type()?.is_dir() {
                aux(filters, &entry.path(), list, state, ignore)?;
            } else {
                let path_string = path.to_str().unwrap().to_string();
                for filter in filters {
                    // TODO FEAT case insensitivity
                    if !path_string.contains(filter) {
                        continue 'entry;
                    }
                }
                list.push(ListPickerItem {
                    kind: ListPickerItemKind::Item,
                    label: path_string,
                    command: format!("edit {}", path.to_string_lossy()),
                    filter_text: path.to_string_lossy().to_string(),
                });
            }
        }
        Ok(())
    }

    let mut list = Vec::new();
    let filters = filter.split(' ').collect::<Vec<&str>>();
    aux(&filters, &working_directory, &mut list, state, ignore)?;
    Ok(list)
}

fn file_list_to_file_tree(list: Vec<ListPickerItem>) -> Vec<ListPickerItem> {
    // Build up some kind of prefix tree built on the paths to
    // extract sections and files from a flat file list.

    #[derive(Default)]
    struct Node<'a> {
        part: &'a str,
        children: BTreeMap<&'a str, usize>,
        items: BTreeMap<&'a str, &'a ListPickerItem>,
    }

    let mut nodes = Vec::new();
    nodes.push(Node::default());

    for item in &list {
        let path = &item.label;

        let mut parts = path.split('/').peekable();
        let mut curr_node_id = 0;
        while let Some(part) = parts.next() {
            if parts.peek().is_none() {
                let curr_node = &mut nodes[curr_node_id];
                curr_node.items.insert(part, item);
            } else {
                if !nodes[curr_node_id].children.contains_key(part) {
                    let next_node_id = nodes.len();
                    nodes.push(Node {
                        part,
                        ..Default::default()
                    });
                    nodes[curr_node_id].children.insert(part, next_node_id);
                    curr_node_id = next_node_id;
                } else {
                    let next_node_id = *nodes[curr_node_id].children.get(part).unwrap();
                    curr_node_id = next_node_id;
                }
            }
        }
    }

    // Extract sections and files
    fn aux(
        curr_node: usize,
        nodes: &[Node],
        parts: &str,
        level: i32,
        out: &mut Vec<ListPickerItem>,
    ) {
        const IDENT_SIZE: i32 = 2; // TODO make configurable

        let node = &nodes[curr_node];
        let mut next_parts = parts.to_string();
        let mut next_level = level;
        if node.part != "" {
            if node.items.is_empty() {
                next_parts = format!("{parts}{}/", node.part);
            } else {
                next_parts = format!("");
                next_level += 1;
                let indent = " ".repeat((level * IDENT_SIZE) as _);
                let dir_path = parts.to_string();
                out.push(ListPickerItem {
                    kind: ListPickerItemKind::Section,
                    label: format!("{indent}{parts}{}/", node.part),
                    command: String::new(),
                    filter_text: dir_path,
                });
            }
        }

        for (part, item) in &node.items {
            let indent = " ".repeat((next_level * IDENT_SIZE) as _);
            out.push(ListPickerItem {
                kind: ListPickerItemKind::Item,
                label: format!("{indent}{part}"),
                command: item.command.clone(),
                filter_text: item.filter_text.clone(),
            });
        }

        for &child in node.children.values() {
            aux(child, nodes, &next_parts, next_level, out);
        }
    }

    let mut out = Vec::new();
    aux(0, &nodes, "", 0, &mut out);

    return out;
}

// == Diagnostics picker stuff ==

fn register_diagnostics_picker_commands(cr: &mut CommandRegistry) {
    cr.register(
        "diagnostics-picker-filter-list",
        "nodoc",
        focused_buffer_command(|_opt, ctx| {
            let filter = ctx.buffer.line(0).unwrap_or_default();
            let filters: Vec<&str> = filter.split_ascii_whitespace().collect();

            use DiagnosticKind as Dk;

            let mut kind_buckets = HashMap::<Dk, Vec<(&Path, &Diagnostic)>>::new();
            let mut previous_kind = Dk::Lint; // randomly picked idc
            'bucket: for (path, diag) in ctx.state.diagnostics.iter() {
                // Does filtering this even makes sense?
                for filter in &filters {
                    if !diag.message.contains(filter) {
                        continue 'bucket;
                    }
                }
                // Lump ExtraInfos with the previous entry.
                let kind = if diag.kind == Dk::ExtraInfo {
                    previous_kind
                } else {
                    previous_kind = diag.kind;
                    diag.kind
                };

                kind_buckets.entry(kind).or_default().push((path, diag));
            }

            let mut filtered_list = Vec::new();
            for kind in [Dk::Error, Dk::Warning, Dk::Lint] {
                let bucket = kind_buckets.remove(&kind).unwrap_or_default();
                if bucket.is_empty() {
                    continue;
                }
                filtered_list.push(ListPickerItem {
                    kind: ListPickerItemKind::Section,
                    label: format!("{kind:?}s"),
                    command: String::new(),
                    filter_text: String::new(),
                });
                for (path, diag) in bucket {
                    let command = format!(
                        "edit {}:{}",
                        path.to_string_lossy(),
                        diag.range.start.offset((1, 1))
                    );
                    filtered_list.push(ListPickerItem {
                        kind: ListPickerItemKind::Item,
                        label: format!("  {}", diag.message),
                        command: command,
                        filter_text: String::new(),
                    });
                }
            }

            ctx.state.list_picker.items = filtered_list;
            ctx.state.list_picker.reselect();
            Ok(())
        }),
    );
}
