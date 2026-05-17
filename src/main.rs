use macroquad::prelude::*;
use macroquad::ui::{root_ui, widgets, hash}; // hash を追加
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::{Read, Write};

// --- 1. テーマ管理 ---
struct AppTheme {
    bg: Color,
    grid_frame: Color,
    cell: Color,
    text: Color,
    yes: Color,
    no: Color,
    highlight: Color,
}

impl AppTheme {
    fn dark() -> Self {
        Self {
            bg: Color::from_rgba(28, 28, 33, 255),
            grid_frame: Color::from_rgba(76, 76, 76, 255),
            cell: Color::from_rgba(46, 46, 46, 255),
            text: WHITE,
            yes: SKYBLUE, // CYAN から SKYBLUE に修正
            no: ORANGE,
            highlight: Color::from_rgba(255, 255, 255, 13),
        }
    }
    fn light() -> Self {
        Self {
            bg: Color::from_rgba(242, 242, 242, 255),
            grid_frame: Color::from_rgba(178, 178, 178, 255),
            cell: WHITE,
            text: Color::from_rgba(25, 25, 25, 255),
            yes: RED,
            no: BLUE,
            highlight: Color::from_rgba(0, 0, 0, 13),
        }
    }
}

// --- 2. 履歴・セーブ管理 ---
#[derive(Serialize, Deserialize, Clone)]
struct SaveData {
    grid_data: Vec<Vec<i8>>,
    elapsed: f32,
}

struct HistoryManager {
    undo_stack: Vec<Vec<Vec<i8>>>,
    redo_stack: Vec<Vec<Vec<i8>>>,
}

impl HistoryManager {
    fn new() -> Self {
        Self { undo_stack: Vec::new(), redo_stack: Vec::new() }
    }
    fn record(&mut self, g: &Vec<Vec<i8>>) {
        if let Some(last) = self.undo_stack.last() {
            if last == g { return; }
        }
        self.undo_stack.push(g.clone());
        if self.undo_stack.len() > 50 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }
    fn undo(&mut self, g: &mut Vec<Vec<i8>>) {
        if self.undo_stack.is_empty() { return; }
        self.redo_stack.push(g.clone());
        *g = self.undo_stack.pop().unwrap();
    }
    fn redo(&mut self, g: &mut Vec<Vec<i8>>) {
        if self.redo_stack.is_empty() { return; }
        self.undo_stack.push(g.clone());
        *g = self.redo_stack.pop().unwrap();
    }
}

// --- 3. 推論エンジン & 演出 ---
struct LogicEffect {
    pos: Vec2,
    color: Color,
    time: f32,
}

struct InferenceEngine {
    rel: Vec<Vec<i8>>,
    size: usize,
}

impl InferenceEngine {
    fn new(n: usize) -> Self {
        let mut rel = vec![vec![0; n]; n];
        for i in 0..n { rel[i][i] = 1; }
        Self { rel, size: n }
    }

    fn set_relation(&mut self, a: usize, b: usize, s: i8, changed_cells: &mut Vec<(usize, usize, i8)>) {
        if self.rel[a][b] == s { return; }
        self.rel[a][b] = s;
        self.rel[b][a] = s;
        changed_cells.push((a, b, s));

        for k in 0..self.size {
            for i in 0..self.size {
                for j in 0..self.size {
                    if self.rel[i][j] != 0 { continue; }
                    if self.rel[i][k] == 1 && self.rel[k][j] == 1 {
                        self.set_relation(i, j, 1, changed_cells);
                    } else if self.rel[i][k] == 1 && self.rel[k][j] == 2 {
                        self.set_relation(i, j, 2, changed_cells);
                    }
                }
            }
        }
    }
}

fn ease_out_expo(t: f32) -> f32 {
    if t == 1.0 { 1.0 } else { 1.0 - f32::powf(2.0, -10.0 * t) }
}

fn get_cell_position(a: usize, b: usize, base: Vec2, cell_size: f32) -> Vec2 {
    let mut cx = a / 3;
    let mut cy = b / 3;
    let mut ix = a % 3;
    let mut iy = b % 3;
    if cx < cy {
        std::mem::swap(&mut cx, &mut cy);
        std::mem::swap(&mut ix, &mut iy);
    }
    base + Vec2::new(
        (cx - 1) as f32 * 135.0 + ix as f32 * cell_size + cell_size * 0.5,
        cy as f32 * 135.0 + iy as f32 * cell_size + cell_size * 0.5
    )
}

// --- 4. メイン ---
fn window_conf() -> Conf {
    Conf {
        window_title: "Logic Matrix Pro".to_owned(),
        window_width: 1280,
        window_height: 720,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut theme = AppTheme::dark();
    let mut is_dark = true;
    let mut elapsed_time = 0.0;
    let mut history = HistoryManager::new();
    let mut engine = InferenceEngine::new(9);
    let mut effects: Vec<LogicEffect> = Vec::new();

    let cell_size: f32 = 45.0;
    let base = Vec2::new(250.0, 180.0);

    if let Ok(mut file) = File::open("save.dat") {
        let mut buffer = Vec::new();
        if file.read_to_end(&mut buffer).is_ok() {
            if let Ok(decoded) = bincode::deserialize::<SaveData>(&buffer) {
                engine.rel = decoded.grid_data;
                elapsed_time = decoded.elapsed;
            }
        }
    }

    loop {
        let dt = get_frame_time();
        elapsed_time += dt;

        if is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl) {
            if is_key_pressed(KeyCode::Z) { history.undo(&mut engine.rel); }
            if is_key_pressed(KeyCode::Y) { history.redo(&mut engine.rel); }
        }

        clear_background(theme.bg);

        // --- UIレイアウト (右上の設定用透明ウィンドウ) ---
        let old_is_dark = is_dark;
        root_ui().window(hash!(), Vec2::new(1040.0, 15.0), Vec2::new(220.0, 45.0), |ui| {
            widgets::Checkbox::new(hash!())
                .label("ダークモード")
                .ui(ui, &mut is_dark);
        });
        
        // チェックボックスの状態が変わったらテーマを更新
        if is_dark != old_is_dark {
            theme = if is_dark { AppTheme::dark() } else { AppTheme::light() };
        }

        let (mouse_x, mouse_y) = mouse_position();

        // UIウィンドウの上にマウスがないときだけ盤面クリックを受け付ける
        let mouse_in_ui = mouse_x >= 1040.0 && mouse_y <= 60.0;

        // --- グリッドの描画とクリック処理 ---
        for cy in 0..2 {
            for cx in (cy + 1)..3 {
                let p = base + Vec2::new((cx - 1) as f32 * 135.0, cy as f32 * 135.0);
                for y in 0..3 {
                    for x in 0..3 {
                        let cell_x = p.x + x as f32 * cell_size;
                        let cell_y = p.y + y as f32 * cell_size;
                        let id_x = cx * 3 + x;
                        let id_y = cy * 3 + y;
                        let s = engine.rel[id_y][id_x];

                        let mouse_over = !mouse_in_ui && 
                                         mouse_x >= cell_x && mouse_x < cell_x + cell_size &&
                                         mouse_y >= cell_y && mouse_y < cell_y + cell_size;

                        if mouse_over && is_mouse_button_pressed(MouseButton::Left) {
                            history.record(&engine.rel);
                            let next_s = match s { 1 => 2, 2 => 0, _ => 1 };
                            
                            let mut changed_cells = Vec::new();
                            engine.set_relation(id_x, id_y, next_s, &mut changed_cells);

                            for (a, b, state) in changed_cells {
                                if state != 0 {
                                    effects.push(LogicEffect {
                                        pos: get_cell_position(a, b, base, cell_size),
                                        color: if state == 1 { theme.yes } else { theme.no },
                                        time: 0.0,
                                    });
                                }
                            }
                        }

                        draw_rectangle(cell_x, cell_y, cell_size, cell_size, theme.cell);
                        draw_rectangle_lines(cell_x, cell_y, cell_size, cell_size, 1.0, theme.grid_frame);

                        let center_x = cell_x + cell_size * 0.5;
                        let center_y = cell_y + cell_size * 0.5;

                        if s == 1 {
                            draw_circle_lines(center_x, center_y, 15.0, 3.0, theme.yes);
                        } else if s == 2 {
                            draw_line(cell_x + 10.0, cell_y + 10.0, cell_x + cell_size - 10.0, cell_y + cell_size - 10.0, 3.0, theme.no);
                            draw_line(cell_x + cell_size - 10.0, cell_y + 10.0, cell_x + 10.0, cell_y + cell_size - 10.0, 3.0, theme.no);
                        }

                        if mouse_over {
                            draw_rectangle(cell_x, cell_y, cell_size, cell_size, theme.highlight);
                        }
                    }
                }
            }
        }

        // --- エフェクトの更新と描画 ---
        effects.retain_mut(|eff| {
            eff.time += dt;
            let size = ease_out_expo(eff.time) * 40.0;
            let mut alpha = 1.0 - eff.time;
            if alpha < 0.0 { alpha = 0.0; }
            
            let mut c = eff.color;
            c.a = alpha;
            draw_circle_lines(eff.pos.x, eff.pos.y, size, 2.0, c);
            
            eff.time < 1.0
        });

        // --- タイトル、タイマー、ボタンUI ---
        draw_text("Logic Matrix Pro", 40.0, 60.0, 40.0, theme.yes);

        let minutes = (elapsed_time / 60.0) as i32;
        let seconds = (elapsed_time % 60.0) as i32;
        let millis = ((elapsed_time % 1.0) * 100.0) as i32;
        let time_str = format!("Time: {:02}:{:02}.{:02}", minutes, seconds, millis);
        draw_text(&time_str, 1050.0, 80.0, 20.0, theme.text);

        if root_ui().button(Vec2::new(850.0, 600.0), "Undo") {
            history.undo(&mut engine.rel);
        }
        if root_ui().button(Vec2::new(940.0, 600.0), "Redo") {
            history.redo(&mut engine.rel);
        }
        if root_ui().button(Vec2::new(1050.0, 600.0), "セーブ") {
            let data = SaveData { grid_data: engine.rel.clone(), elapsed: elapsed_time };
            if let Ok(encoded) = bincode::serialize(&data) {
                if let Ok(mut file) = File::create("save.dat") {
                    let _ = file.write_all(&encoded);
                }
            }
        }

        next_frame().await
    }
}