use super::*;

pub(crate) struct BoardUi {
    pub(crate) board_root: UiNodeId,
    pub(crate) main_menu_root: UiNodeId,
    pub(crate) menu_root: UiNodeId,
    pub(crate) cells: [[UiNodeId; 9]; 9],
    pub(crate) labels: [[UiNodeId; 9]; 9],
    pub(crate) status: UiNodeId,
}

pub(crate) fn build_board(ui: &mut UiSubsystem, config: UiConfig, puzzle: [[u8; 9]; 9]) -> BoardUi {
    let tree = ui.tree_mut();
    let mut root = UiNodeContent::new(UiNodeKind::Panel);
    root.style = UiStyle {
        width: UiLength::Percent(100.0),
        height: UiLength::Percent(100.0),
        background: [0.965, 0.972, 0.988, 1.0],
        ..UiStyle::default()
    };
    let root = tree.create(tree.root(), root).expect("create game root");

    let mut title = UiNodeContent::new(UiNodeKind::Text);
    title.style = UiStyle {
        width: UiLength::Points(576.0),
        height: UiLength::Points(42.0),
        position: UiPosition::Absolute {
            left: 40.0,
            top: 36.0,
        },
        ..UiStyle::default()
    };
    title.text = Some(UiText::try_new("SUDOKU", config.max_text_bytes).expect("static title"));
    tree.create(root, title).expect("create Sudoku title");

    let mut main_menu = UiNodeContent::new(UiNodeKind::Panel);
    main_menu.style = UiStyle {
        width: UiLength::Points(280.0),
        height: UiLength::Points(280.0),
        direction: rust_kernel_game_engine_kit::subsystems::ui::UiDirection::Column,
        gap: 14.0,
        position: UiPosition::Absolute {
            left: 310.0,
            top: 280.0,
        },
        ..UiStyle::default()
    };
    let main_menu_root = tree.create(root, main_menu).expect("create main menu");
    for (label, color) in [
        ("Continue", [0.25, 0.52, 0.95, 1.0]),
        ("New Game", [0.96, 0.97, 0.99, 1.0]),
        ("Leaderboard", [0.96, 0.97, 0.99, 1.0]),
        ("Quit", [0.95, 0.25, 0.28, 1.0]),
    ] {
        let mut button = UiNodeContent::new(UiNodeKind::Panel);
        button.style = UiStyle {
            width: UiLength::Percent(100.0),
            height: UiLength::Points(56.0),
            padding: UiEdges::all(16.0),
            background: color,
            border: [0.72, 0.78, 0.88, 1.0],
            border_width: 1.0,
            corner_radius: 10.0,
            ..UiStyle::default()
        };
        button.interaction.focusable = true;
        let button = tree
            .create(main_menu_root, button)
            .expect("create menu button");
        let mut text = UiNodeContent::new(UiNodeKind::Text);
        text.style = UiStyle {
            width: UiLength::Percent(100.0),
            height: UiLength::Points(28.0),
            ..UiStyle::default()
        };
        text.text = Some(UiText::try_new(label, config.max_text_bytes).expect("menu label"));
        tree.create(button, text).expect("create menu label");
    }

    let mut menu = UiNodeContent::new(UiNodeKind::Panel);
    menu.visible = false;
    menu.style = UiStyle {
        width: UiLength::Points(756.0),
        height: UiLength::Points(280.0),
        direction: rust_kernel_game_engine_kit::subsystems::ui::UiDirection::Row,
        gap: 24.0,
        position: UiPosition::Absolute {
            left: 96.0,
            top: 280.0,
        },
        ..UiStyle::default()
    };
    let menu = tree.create(root, menu).expect("create difficulty menu");
    for (index, (label, color)) in [
        ("EASY", [0.10, 0.55, 0.30, 1.0]),
        ("MEDIUM", [0.80, 0.50, 0.06, 1.0]),
        ("HARD", [0.78, 0.14, 0.12, 1.0]),
    ]
    .into_iter()
    .enumerate()
    {
        let mut card = UiNodeContent::new(UiNodeKind::Panel);
        card.style = UiStyle {
            width: UiLength::Points(220.0),
            height: UiLength::Points(280.0),
            padding: UiEdges::all(20.0),
            background: color,
            border: [0.9, 0.9, 0.9, 0.35],
            border_width: 2.0,
            corner_radius: 12.0,
            ..UiStyle::default()
        };
        card.interaction.focusable = true;
        let card = tree.create(menu, card).expect("create difficulty card");
        let mut text = UiNodeContent::new(UiNodeKind::Text);
        text.style = UiStyle {
            width: UiLength::Percent(100.0),
            height: UiLength::Points(36.0),
            ..UiStyle::default()
        };
        text.text = Some(UiText::try_new(label, config.max_text_bytes).expect("static difficulty"));
        tree.create(card, text).expect("create difficulty label");
        let mut detail = UiNodeContent::new(UiNodeKind::Text);
        detail.style = UiStyle {
            width: UiLength::Percent(100.0),
            height: UiLength::Points(72.0),
            ..UiStyle::default()
        };
        let description = match index {
            0 => "46-50 blanks | 3 lives | 3 hints",
            1 => "51-55 blanks | 3 lives | 1 hint",
            _ => "56-64 blanks | 3 lives | no hints",
        };
        detail.text =
            Some(UiText::try_new(description, config.max_text_bytes).expect("static description"));
        tree.create(card, detail).expect("create difficulty detail");
    }

    let mut board = UiNodeContent::new(UiNodeKind::Panel);
    board.visible = false;
    board.style = UiStyle {
        width: UiLength::Points(576.0),
        height: UiLength::Points(576.0),
        direction: rust_kernel_game_engine_kit::subsystems::ui::UiDirection::Column,
        gap: 0.0,
        overflow: rust_kernel_game_engine_kit::subsystems::ui::UiOverflow::Clip,
        background: [0.07, 0.10, 0.18, 1.0],
        position: UiPosition::Absolute {
            left: 40.0,
            top: 110.0,
        },
        ..UiStyle::default()
    };
    let board = tree.create(root, board).expect("create Sudoku board");

    let mut cells = [[UiNodeId::new(0, 0); 9]; 9];
    let mut labels = [[UiNodeId::new(0, 0); 9]; 9];
    for (row_index, row) in puzzle.into_iter().enumerate() {
        let mut row_content = UiNodeContent::new(UiNodeKind::Panel);
        row_content.style = UiStyle {
            width: UiLength::Percent(100.0),
            height: UiLength::Points(64.0),
            direction: rust_kernel_game_engine_kit::subsystems::ui::UiDirection::Row,
            gap: 0.0,
            ..UiStyle::default()
        };
        let row_node = tree.create(board, row_content).expect("create Sudoku row");
        for (column_index, value) in row.into_iter().enumerate() {
            let mut cell = UiNodeContent::new(UiNodeKind::Panel);
            cell.style = UiStyle {
                width: UiLength::Points(64.0),
                height: UiLength::Points(64.0),
                direction: rust_kernel_game_engine_kit::subsystems::ui::UiDirection::Column,
                align: UiAlign::Center,
                padding: UiEdges::all(12.0),
                background: if value == 0 {
                    [0.99, 0.99, 1.0, 1.0]
                } else {
                    [0.89, 0.92, 0.96, 1.0]
                },
                border: [0.72, 0.78, 0.86, 1.0],
                border_width: 1.0,
                ..UiStyle::default()
            };
            cell.interaction.focusable = true;
            let cell_node = tree.create(row_node, cell).expect("create Sudoku cell");
            cells[row_index][column_index] = cell_node;

            let mut label = UiNodeContent::new(UiNodeKind::Text);
            label.style = UiStyle {
                width: UiLength::Percent(100.0),
                height: UiLength::Points(30.0),
                background: [0.0, 0.0, 0.0, 0.0],
                ..UiStyle::default()
            };
            label.text = Some(
                UiText::try_new(
                    if value == 0 { "" } else { digit_text(value) },
                    config.max_text_bytes,
                )
                .expect("static Sudoku digit is within UI text budget"),
            );
            labels[row_index][column_index] =
                tree.create(cell_node, label).expect("create Sudoku label");
        }
    }

    let mut status = UiNodeContent::new(UiNodeKind::Text);
    status.style = UiStyle {
        width: UiLength::Points(576.0),
        height: UiLength::Points(32.0),
        margin: UiEdges::all(12.0),
        ..UiStyle::default()
    };
    status.text = Some(
        UiText::try_new(
            "Select a cell | 1-9 enter | N notes | H hint | P pause",
            config.max_text_bytes,
        )
        .expect("static status is within UI text budget"),
    );
    let status = tree.create(root, status).expect("create Sudoku status");

    // Keep the command capacity explicit even though this static example does
    // not mutate the board yet; gameplay mutations use UiCommandBuffer at the
    // same deferred boundary as the production UI path.
    assert!(ui.commands().capacity() >= config.max_commands);
    BoardUi {
        board_root: board,
        main_menu_root,
        menu_root: menu,
        cells,
        labels,
        status,
    }
}
