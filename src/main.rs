use gtk4 as gtk;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, Button, DrawingArea, 
    Orientation, Scale, Frame,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use rand::Rng;

// Initial grid dimensions that will grow over time
const CELL_SIZE: i32 = 8;  // Cell size in pixels
const INITIAL_GRID_WIDTH: usize = 150;  // Initial width
const INITIAL_GRID_HEIGHT: usize = 100; // Initial height
const MAX_GRID_WIDTH: usize = 5000;      // Maximum width the grid can grow to
const MAX_GRID_HEIGHT: usize = 3000;     // Maximum height the grid can grow to
const GROWTH_INTERVAL: u64 = 50;        // How many updates before growing the grid
const GROWTH_AMOUNT: usize = 1;         // How many cells to add in each direction when growing

// Game state
struct GameState {
    grid: Vec<Vec<bool>>,
    grid_width: usize,
    grid_height: usize,
    running: bool,
    speed: u64, // in milliseconds
    update_counter: u64, // Count updates for growth timing
    auto_grow: bool, // Whether to auto-grow the universe
    generation_count: u64, // Track number of generations
    living_cells: usize, // Track number of living cells
    cell_births: u64, // Track cell births
    cell_deaths: u64, // Track cell deaths
    timeout_id: Option<gtk::glib::SourceId>, // Store current timeout ID
}

impl GameState {
    fn new() -> Self {
        // Initialize with an empty grid
        let grid = vec![vec![false; INITIAL_GRID_WIDTH]; INITIAL_GRID_HEIGHT];

        Self { 
            grid,
            grid_width: INITIAL_GRID_WIDTH,
            grid_height: INITIAL_GRID_HEIGHT,
            running: false,
            speed: 100,
            update_counter: 0,
            auto_grow: true,
            generation_count: 0,
            living_cells: 0,
            cell_births: 0,
            cell_deaths: 0,
            timeout_id: None,
        }
    }

    // Calculate and update statistics
    fn update_statistics(&mut self) {
        let mut count = 0;
        for row in &self.grid {
            for &cell in row {
                if cell {
                    count += 1;
                }
            }
        }
        self.living_cells = count;
    }

    fn add_glider(&mut self, x: usize, y: usize) {
        if x + 2 < self.grid_width && y + 2 < self.grid_height {
            self.grid[y][x+1] = true;
            self.grid[y+1][x+2] = true;
            self.grid[y+2][x] = true;
            self.grid[y+2][x+1] = true;
            self.grid[y+2][x+2] = true;
        }
        self.update_statistics();
    }

    // Add a cool pattern: Gosper's Glider Gun
    fn add_glider_gun(&mut self, x: usize, y: usize) {
        if x + 36 < self.grid_width && y + 9 < self.grid_height {
            // Left block
            self.grid[y+4][x+0] = true;
            self.grid[y+4][x+1] = true;
            self.grid[y+5][x+0] = true;
            self.grid[y+5][x+1] = true;

            // Left gun
            self.grid[y+2][x+12] = true;
            self.grid[y+2][x+13] = true;
            self.grid[y+3][x+11] = true;
            self.grid[y+3][x+15] = true;
            self.grid[y+4][x+10] = true;
            self.grid[y+4][x+16] = true;
            self.grid[y+5][x+10] = true;
            self.grid[y+5][x+14] = true;
            self.grid[y+5][x+16] = true;
            self.grid[y+5][x+17] = true;
            self.grid[y+6][x+10] = true;
            self.grid[y+6][x+16] = true;
            self.grid[y+7][x+11] = true;
            self.grid[y+7][x+15] = true;
            self.grid[y+8][x+12] = true;
            self.grid[y+8][x+13] = true;

            // Right block
            self.grid[y+2][x+24] = true;
            self.grid[y+2][x+25] = true;
            self.grid[y+3][x+24] = true;
            self.grid[y+3][x+25] = true;
        }
        self.update_statistics();
    }

    // Add a spaceship
    fn add_spaceship(&mut self, x: usize, y: usize) {
        if x + 4 < self.grid_width && y + 3 < self.grid_height {
            self.grid[y][x+1] = true;
            self.grid[y][x+4] = true;
            self.grid[y+1][x] = true;
            self.grid[y+2][x] = true;
            self.grid[y+2][x+4] = true;
            self.grid[y+3][x] = true;
            self.grid[y+3][x+1] = true;
            self.grid[y+3][x+2] = true;
            self.grid[y+3][x+3] = true;
        }
        self.update_statistics();
    }

    // Add a pulsar pattern (period 3 oscillator)
    fn add_pulsar(&mut self, x: usize, y: usize) {
        if x + 12 < self.grid_width && y + 12 < self.grid_height {
            // Outer vertical lines
            for dy in [0, 5, 7, 12] {
                for dx in [2, 3, 4, 8, 9, 10] {
                    self.grid[y+dy][x+dx] = true;
                }
            }

            // Outer horizontal lines
            for dx in [0, 5, 7, 12] {
                for dy in [2, 3, 4, 8, 9, 10] {
                    self.grid[y+dy][x+dx] = true;
                }
            }
        }
        self.update_statistics();
    }

    // Add an R-pentomino (chaotic pattern that evolves for a long time)
    fn add_r_pentomino(&mut self, x: usize, y: usize) {
        if x + 2 < self.grid_width && y + 2 < self.grid_height {
            self.grid[y][x+1] = true;
            self.grid[y][x+2] = true;
            self.grid[y+1][x] = true;
            self.grid[y+1][x+1] = true;
            self.grid[y+2][x+1] = true;
        }
        self.update_statistics();
    }

    fn randomize(&mut self) {
        let mut rng = rand::thread_rng();
        for y in 0..self.grid_height {
            for x in 0..self.grid_width {
                // About 20% chance for a cell to be alive (less dense for better patterns)
                self.grid[y][x] = rng.gen_bool(0.2);
            }
        }
        self.update_statistics();
    }

    // Create random pattern in center region only
    fn randomize_center(&mut self) {
        let mut rng = rand::thread_rng();

        // Clear the grid first
        self.clear();

        // Determine the center region (about 1/4 of the total area)
        let start_x = self.grid_width / 3;
        let end_x = self.grid_width * 2 / 3;
        let start_y = self.grid_height / 3;
        let end_y = self.grid_height * 2 / 3;

        // Fill only the center region
        for y in start_y..end_y {
            for x in start_x..end_x {
                // About 30% chance for a cell to be alive in the center region
                self.grid[y][x] = rng.gen_bool(0.3);
            }
        }

        self.update_statistics();
    }

    fn clear(&mut self) {
        for y in 0..self.grid_height {
            for x in 0..self.grid_width {
                self.grid[y][x] = false;
            }
        }

        // Reset counters
        self.update_counter = 0;
        self.generation_count = 0;
        self.cell_births = 0;
        self.cell_deaths = 0;
        self.living_cells = 0;
    }

    fn toggle_cell(&mut self, x: usize, y: usize) {
        if x < self.grid_width && y < self.grid_height {
            self.grid[y][x] = !self.grid[y][x];

            // Update statistics
            if self.grid[y][x] {
                self.living_cells += 1;
                self.cell_births += 1;
            } else {
                self.living_cells -= 1;
                self.cell_deaths += 1;
            }
        }
    }

    fn grow_universe(&mut self) {
        // Only grow if we're below the maximum size
        if self.grid_width >= MAX_GRID_WIDTH || self.grid_height >= MAX_GRID_HEIGHT {
            return;
        }

        // Calculate new dimensions
        let new_width = (self.grid_width + GROWTH_AMOUNT).min(MAX_GRID_WIDTH);
        let new_height = (self.grid_height + GROWTH_AMOUNT).min(MAX_GRID_HEIGHT);

        // Create a new grid with expanded dimensions
        let mut new_grid = vec![vec![false; new_width]; new_height];

        // Copy existing data to the center of the new grid
        let x_offset = (new_width - self.grid_width) / 2;
        let y_offset = (new_height - self.grid_height) / 2;

        for y in 0..self.grid_height {
            for x in 0..self.grid_width {
                new_grid[y + y_offset][x + x_offset] = self.grid[y][x];
            }
        }

        // Update grid
        self.grid = new_grid;
        self.grid_width = new_width;
        self.grid_height = new_height;

        // Report growth
        println!("Universe expanded to {}x{}", self.grid_width, self.grid_height);
    }

    fn update(&mut self) {
        if !self.running {
            return;
        }

        // Check if it's time to grow the universe
        self.update_counter += 1;
        self.generation_count += 1;

        if self.auto_grow && self.update_counter % GROWTH_INTERVAL == 0 {
            self.grow_universe();
        }

        let mut new_grid = vec![vec![false; self.grid_width]; self.grid_height];
        let mut births = 0;
        let mut deaths = 0;

        for y in 0..self.grid_height {
            for x in 0..self.grid_width {
                let alive_neighbors = self.count_alive_neighbors(x, y);
                let cell_alive = self.grid[y][x];

                new_grid[y][x] = match (cell_alive, alive_neighbors) {
                    (true, 0..=1) => {
                        deaths += 1;
                        false  // Underpopulation
                    },
                    (true, 2..=3) => true,   // Survival
                    (true, _) => {
                        deaths += 1;
                        false  // Overpopulation
                    },
                    (false, 3) => {
                        births += 1;
                        true   // Reproduction
                    },
                    (state, _) => state,     // No change
                };
            }
        }

        self.cell_births += births;
        self.cell_deaths += deaths;
        self.grid = new_grid;
        self.update_statistics();
    }

    fn count_alive_neighbors(&self, x: usize, y: usize) -> usize {
        let mut count = 0;

        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue; // Skip the cell itself
                }

                let nx = (x as isize + dx).rem_euclid(self.grid_width as isize) as usize;
                let ny = (y as isize + dy).rem_euclid(self.grid_height as isize) as usize;

                if self.grid[ny][nx] {
                    count += 1;
                }
            }
        }

        count
    }
}

// UI state wrapper to avoid ownership issues
struct UiState {
    game_state: Rc<RefCell<GameState>>,
    drawing_area: DrawingArea,
    generation_label: gtk::Label,
    cells_label: gtk::Label,
    size_label: gtk::Label,
    birth_death_label: gtk::Label,
    coord_label: gtk::Label,
}

impl UiState {
    fn new(game_state: Rc<RefCell<GameState>>) -> Self {
        Self {
            game_state,
            drawing_area: DrawingArea::new(),
            generation_label: gtk::Label::new(Some("Generation: 0")),
            cells_label: gtk::Label::new(Some("Living Cells: 0")),
            size_label: gtk::Label::new(Some(&format!("Universe: {}x{}", INITIAL_GRID_WIDTH, INITIAL_GRID_HEIGHT))),
            birth_death_label: gtk::Label::new(Some("Births: 0  Deaths: 0")),
            coord_label: gtk::Label::new(Some("Coordinates: -,-")),
        }
    }

    fn update_statistics(&self) {
        let state = self.game_state.borrow();
        self.generation_label.set_text(&format!("Generation: {}", state.generation_count));
        self.cells_label.set_text(&format!("Living Cells: {}", state.living_cells));
        self.size_label.set_text(&format!("Universe: {}x{}", state.grid_width, state.grid_height));
        self.birth_death_label.set_text(&format!("Births: {}  Deaths: {}", state.cell_births, state.cell_deaths));
    }

    fn setup_game_loop(&self, speed: u64) -> gtk::glib::SourceId {
        let game_state = self.game_state.clone();
        let drawing_area = self.drawing_area.clone();
        let ui_state = Rc::new(self.clone());

        gtk::glib::timeout_add_local(Duration::from_millis(speed), move || {
            let mut state = game_state.borrow_mut();
            if state.running {
                state.update();
                drop(state);  // Release borrow before UI updates

                // Update UI
                ui_state.update_statistics();
                drawing_area.queue_draw();
                glib::ControlFlow::Continue
            } else {
                glib::ControlFlow::Break
            }
        })
    }
}

// Make UiState cloneable to use in callbacks
impl Clone for UiState {
    fn clone(&self) -> Self {
        Self {
            game_state: self.game_state.clone(),
            drawing_area: self.drawing_area.clone(),
            generation_label: self.generation_label.clone(),
            cells_label: self.cells_label.clone(),
            size_label: self.size_label.clone(),
            birth_death_label: self.birth_death_label.clone(),
            coord_label: self.coord_label.clone(),
        }
    }
}

fn main() {
    let application = Application::new(
        Some("com.example.GameOfLife"),
        Default::default(),
    );

    application.connect_activate(build_ui);
    application.run();
}

fn build_ui(app: &Application) {
    // Create game state
    let game_state = Rc::new(RefCell::new(GameState::new()));

    // Create UI state
    let ui = Rc::new(UiState::new(game_state.clone()));

    // Configure drawing area
    ui.drawing_area.set_content_width(MAX_GRID_WIDTH as i32 * CELL_SIZE);
    ui.drawing_area.set_content_height(MAX_GRID_HEIGHT as i32 * CELL_SIZE);

    // Create scroll window for the drawing area
    let scroll_window = gtk::ScrolledWindow::new();
    scroll_window.set_child(Some(&ui.drawing_area));
    scroll_window.set_min_content_width(1000);
    scroll_window.set_min_content_height(600);
    scroll_window.set_hexpand(true);
    scroll_window.set_vexpand(true);

    // Create control buttons
    let play_button = Button::with_label("Play");
    let clear_button = Button::with_label("Clear");
    let random_button = Button::with_label("Random");
    let center_random_button = Button::with_label("Center Random");
    let grow_button = Button::with_label("Grow Universe");

    // Create pattern buttons
    let glider_button = Button::with_label("Glider");
    let spaceship_button = Button::with_label("Spaceship");
    let glider_gun_button = Button::with_label("Glider Gun");
    let pulsar_button = Button::with_label("Pulsar");
    let r_pentomino_button = Button::with_label("R-Pentomino");

    // Create speed controls
    let speed_label = gtk::Label::new(Some("Speed:"));
    let speed_scale = Scale::with_range(Orientation::Horizontal, 10.0, 500.0, 10.0);
    speed_scale.set_value(100.0); // default speed
    speed_scale.set_width_request(150);
    speed_scale.set_draw_value(true);
    speed_scale.set_value_pos(gtk::PositionType::Right);

    // Create auto-grow checkbox
    let auto_grow_check = gtk::CheckButton::with_label("Auto-grow");
    auto_grow_check.set_active(true);

    // Create button boxes
    let control_box = GtkBox::new(Orientation::Horizontal, 5);
    control_box.append(&play_button);
    control_box.append(&clear_button);
    control_box.append(&random_button);
    control_box.append(&center_random_button);
    control_box.append(&grow_button);
    control_box.append(&speed_label);
    control_box.append(&speed_scale);
    control_box.append(&auto_grow_check);

    // Create patterns box
    let patterns_frame = Frame::new(Some("Patterns"));
    let patterns_box = GtkBox::new(Orientation::Horizontal, 5);
    patterns_box.set_margin_start(5);
    patterns_box.set_margin_end(5);
    patterns_box.set_margin_top(5);
    patterns_box.set_margin_bottom(5);
    patterns_box.append(&glider_button);
    patterns_box.append(&spaceship_button);
    patterns_box.append(&glider_gun_button);
    patterns_box.append(&pulsar_button);
    patterns_box.append(&r_pentomino_button);
    patterns_frame.set_child(Some(&patterns_box));

    // Create stats box
    let stats_frame = Frame::new(Some("Statistics"));
    let stats_box = GtkBox::new(Orientation::Vertical, 5);
    stats_box.set_margin_start(5);
    stats_box.set_margin_end(5);
    stats_box.set_margin_top(5);
    stats_box.set_margin_bottom(5);

    let stats_row1 = GtkBox::new(Orientation::Horizontal, 10);
    stats_row1.append(&ui.generation_label);
    stats_row1.append(&ui.cells_label);

    let stats_row2 = GtkBox::new(Orientation::Horizontal, 10);
    stats_row2.append(&ui.size_label);
    stats_row2.append(&ui.birth_death_label);
    stats_row2.append(&ui.coord_label);

    stats_box.append(&stats_row1);
    stats_box.append(&stats_row2);
    stats_frame.set_child(Some(&stats_box));

    // Create main box
    let main_box = GtkBox::new(Orientation::Vertical, 5);
    main_box.append(&control_box);
    main_box.append(&patterns_frame);
    main_box.append(&stats_frame);
    main_box.append(&scroll_window);
    main_box.set_margin_start(5);
    main_box.set_margin_end(5);
    main_box.set_margin_top(5);
    main_box.set_margin_bottom(5);

    // Set up drawing function
    let game_state_ref = game_state.clone();
    let draw_ui = ui.clone();
    draw_ui.drawing_area.set_draw_func(move |_, cr, width, height| {
        let state = game_state_ref.borrow();

        // Clear background
        cr.set_source_rgb(0.1, 0.1, 0.1);
        cr.paint().unwrap();

        // Calculate the visible grid portion
        let visible_width = (width / CELL_SIZE).min(state.grid_width as i32) as usize;
        let visible_height = (height / CELL_SIZE).min(state.grid_height as i32) as usize;

        // Draw grid lines (only for smaller grid sizes or they become too dense)
        if CELL_SIZE >= 5 {
            cr.set_source_rgb(0.2, 0.2, 0.2);
            cr.set_line_width(0.5);

            // Draw vertical grid lines
            for x in 0..=visible_width {
                cr.move_to(f64::from(x as i32 * CELL_SIZE), 0.0);
                cr.line_to(f64::from(x as i32 * CELL_SIZE), f64::from(visible_height as i32 * CELL_SIZE));
            }

            // Draw horizontal grid lines
            for y in 0..=visible_height {
                cr.move_to(0.0, f64::from(y as i32 * CELL_SIZE));
                cr.line_to(f64::from(visible_width as i32 * CELL_SIZE), f64::from(y as i32 * CELL_SIZE));
            }

            cr.stroke().unwrap();
        }

        // Draw cells
        cr.set_source_rgb(0.8, 0.8, 0.8);
        for y in 0..visible_height {
            for x in 0..visible_width {
                if x < state.grid_width && y < state.grid_height && state.grid[y][x] {
                    if CELL_SIZE >= 5 {
                        // With grid lines, leave a small gap
                        cr.rectangle(
                            f64::from(x as i32 * CELL_SIZE + 1), 
                            f64::from(y as i32 * CELL_SIZE + 1),
                            f64::from(CELL_SIZE - 1),
                            f64::from(CELL_SIZE - 1),
                        );
                    } else {
                        // Without grid lines, fill the whole cell
                        cr.rectangle(
                            f64::from(x as i32 * CELL_SIZE), 
                            f64::from(y as i32 * CELL_SIZE),
                            f64::from(CELL_SIZE),
                            f64::from(CELL_SIZE),
                        );
                    }
                    cr.fill().unwrap();
                }
            }
        }
    });

    // Set up mouse click handler
    let click_ui = ui.clone();
    let click_gesture = gtk::GestureClick::new();
    click_gesture.connect_pressed(move |_gesture, _n_press, x, y| {
        let cell_x = (x / CELL_SIZE as f64) as usize;
        let cell_y = (y / CELL_SIZE as f64) as usize;

        let mut state = click_ui.game_state.borrow_mut();
        if cell_x < state.grid_width && cell_y < state.grid_height {
            state.toggle_cell(cell_x, cell_y);
            drop(state);

            click_ui.update_statistics();
            click_ui.drawing_area.queue_draw();
        }
    });
    ui.drawing_area.add_controller(click_gesture);

    // Set up mouse motion handler
    let motion_ui = ui.clone();
    let motion_controller = gtk::EventControllerMotion::new();
    motion_controller.connect_motion(move |_, x, y| {
        let cell_x = (x / CELL_SIZE as f64) as usize;
        let cell_y = (y / CELL_SIZE as f64) as usize;
        motion_ui.coord_label.set_text(&format!("Coordinates: {},{}", cell_x, cell_y));
    });
    ui.drawing_area.add_controller(motion_controller);

    // Set up play button
    let play_ui = ui.clone();
    play_button.connect_clicked(move |button| {
        let mut state = play_ui.game_state.borrow_mut();
        state.running = !state.running;

        if state.running {
            button.set_label("Pause");

            // Cancel any existing timeout
            if let Some(id) = state.timeout_id.take() {
                id.remove();
            }

            // Set up new game loop with current speed
            let speed = state.speed;
            drop(state);

            let timeout_id = play_ui.setup_game_loop(speed);

            // Store the timeout ID
            play_ui.game_state.borrow_mut().timeout_id = Some(timeout_id);
        } else {
            button.set_label("Play");
            // Cancel the timeout
            if let Some(id) = state.timeout_id.take() {
                id.remove();
            }
        }
    });

    // Set up clear button
    let clear_ui = ui.clone();
    clear_button.connect_clicked(move |_| {
        clear_ui.game_state.borrow_mut().clear();
        clear_ui.update_statistics();
        clear_ui.drawing_area.queue_draw();
    });

    // Set up random button
    let random_ui = ui.clone();
    random_button.connect_clicked(move |_| {
        random_ui.game_state.borrow_mut().randomize();
        random_ui.update_statistics();
        random_ui.drawing_area.queue_draw();
    });

    // Set up center random button
    let center_ui = ui.clone();
    center_random_button.connect_clicked(move |_| {
        center_ui.game_state.borrow_mut().randomize_center();
        center_ui.update_statistics();
        center_ui.drawing_area.queue_draw();
    });

    // Set up glider button
    let glider_ui = ui.clone();
    glider_button.connect_clicked(move |_| {
        glider_ui.game_state.borrow_mut().add_glider(10, 10);
        glider_ui.update_statistics();
        glider_ui.drawing_area.queue_draw();
    });

    // Set up spaceship button
    let spaceship_ui = ui.clone();
    spaceship_button.connect_clicked(move |_| {
        spaceship_ui.game_state.borrow_mut().add_spaceship(20, 20);
        spaceship_ui.update_statistics();
        spaceship_ui.drawing_area.queue_draw();
    });

    // Set up glider gun button
    let gun_ui = ui.clone();
    glider_gun_button.connect_clicked(move |_| {
        gun_ui.game_state.borrow_mut().add_glider_gun(10, 10);
        gun_ui.update_statistics();
        gun_ui.drawing_area.queue_draw();
    });

    // Set up pulsar button
    let pulsar_ui = ui.clone();
    pulsar_button.connect_clicked(move |_| {
        pulsar_ui.game_state.borrow_mut().add_pulsar(30, 20);
        pulsar_ui.update_statistics();
        pulsar_ui.drawing_area.queue_draw();
    });

    // Set up r-pentomino button 
    let r_ui = ui.clone();
    r_pentomino_button.connect_clicked(move |_| {
        // Cache the current width and height
        let grid_width;
        let grid_height;
        {
            let state = r_ui.game_state.borrow();
            grid_width = state.grid_width;
            grid_height = state.grid_height;
        }

        // Now use the cached values
        r_ui.game_state.borrow_mut().add_r_pentomino(grid_width / 2, grid_height / 2);
        r_ui.update_statistics();
        r_ui.drawing_area.queue_draw();
    });

    // Set up grow button
    let grow_ui = ui.clone();
    grow_button.connect_clicked(move |_| {
        grow_ui.game_state.borrow_mut().grow_universe();
        grow_ui.update_statistics();
        grow_ui.drawing_area.queue_draw();
    });

    // Set up auto-grow checkbox
    let auto_ui = ui.clone();
    auto_grow_check.connect_toggled(move |check| {
        auto_ui.game_state.borrow_mut().auto_grow = check.is_active();
    });

    // Set up speed slider
    let speed_ui = ui.clone();
    speed_scale.connect_value_changed(move |scale| {
        let speed = scale.value() as u64;
        let mut state = speed_ui.game_state.borrow_mut();
        state.speed = speed;

        // If currently running, update the timeout with new speed
        if state.running {
            // Cancel current timeout
            if let Some(id) = state.timeout_id.take() {
                id.remove();
            }

            drop(state); // Release borrow before creating new timeout

            // Create a new timeout with the new speed
            let timeout_id = speed_ui.setup_game_loop(speed);

            // Store the new timeout ID
            speed_ui.game_state.borrow_mut().timeout_id = Some(timeout_id);
        }
    });

    // Create window
    let window = ApplicationWindow::new(app);
    window.set_title(Some("Conway's Game of Life - Pattern Explorer"));
    window.set_default_size(1024, 768);
    window.set_child(Some(&main_box));
    window.present();
}
