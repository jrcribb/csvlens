#[allow(dead_code)]
mod util;
mod csv;

extern crate csv as sushi_csv;
use crate::util::events::{Event, Events};

use std::io;
use std::env;
use std::usize;
use tui::Terminal;
use tui::backend::TermionBackend;
use tui::widgets::Widget;
use tui::widgets::{StatefulWidget, Block, Borders};
use tui::buffer::Buffer;
use tui::layout::Rect;
use tui::text::Span;
use tui::style::{Style, Modifier, Color};
use termion::{raw::IntoRawMode, screen::AlternateScreen, event::Key};

#[derive(Debug)]
pub struct CsvTable<'a> {
    header: Vec<String>,
    rows: &'a Vec<Vec<String>>,
}

impl<'a> CsvTable<'a> {

    fn new(header: &[String], rows: &'a Vec<Vec<String>>) -> Self {
        let _header = header.to_vec();
        Self {
            header: _header,
            rows: rows,
        }
    }

}

impl<'a> CsvTable<'a> {

    fn get_column_widths(&self) -> Vec<u16> {
        let mut column_widths = Vec::new();
        for s in self.header.iter() {
            column_widths.push(s.len() as u16);
        }
        for row in self.rows.iter() {
            for (i, value) in row.iter().enumerate() {
                let v = column_widths.get_mut(i).unwrap();
                let value_len = value.len() as u16;
                if *v < value_len {
                    *v = value_len;
                }
            }
        }
        for w in column_widths.iter_mut() {
            *w = *w + 4;
        }
        column_widths
    }

    fn render_row_numbers(
        &self,
        buf: &mut Buffer,
        state: &CsvTableState,
        area: Rect,
        num_rows: usize,
    ) -> u16 {

        // TODO: better to derminte width from total number of records, so this is always fixed
        let max_row_num = state.rows_offset as usize + num_rows + 1;
        let mut section_width = format!("{}", max_row_num).len() as u16;

        // Render line numbers
        let y_first_record = area.y;
        let mut y = area.y;
        for i in 0..num_rows {
            let row_num = i + state.rows_offset as usize + 1;
            let row_num_formatted = format!("{}", row_num);
            let style = Style::default()
                .fg(Color::Rgb(64, 64, 64));
            let span = Span::styled(row_num_formatted, style);
            buf.set_span(0, y, &span, section_width);
            y += 1;
            if y >= area.bottom() {
                break;
            }
        }
        section_width = section_width + 2 + 1;  // one char reserved for line; add one for symmetry

        // Render vertical separator
        let line_number_block = Block::default()
            .borders(Borders::RIGHT)
            .border_style(Style::default().fg(Color::Rgb(64, 64, 64)));
        let line_number_area = Rect::new(
            0,
            y_first_record,
            section_width,
            area.height,
        );
        line_number_block.render(line_number_area, buf);
        section_width = section_width + 2;

        section_width
    }

    fn render_header_borders(&self, buf: &mut Buffer, area: Rect) -> (u16, u16) {
        let block = Block::default()
            .borders(Borders::TOP | Borders::BOTTOM)
            .border_style(Style::default().fg(Color::Rgb(64, 64, 64)));
        let height = 3;
        let area = Rect::new(0, 0, area.width, height);
        block.render(area, buf);
        // y pos of header text and next line
        (height.saturating_sub(2), height)
    }

    fn render_row(
        &self,
        buf: &mut Buffer,
        state: &mut CsvTableState,
        column_widths: &[u16],
        area: Rect,
        x: u16,
        y: u16,
        is_header: bool,
        row: &Vec<String>,
    ) {
        let mut x_offset_header = x;
        let mut remaining_width = area.width.saturating_sub(x);
        let cols_offset = state.cols_offset as usize;
        let mut has_more_cols_to_show = false;
        for (col_index, (hname, &hlen)) in row.iter().zip(column_widths).enumerate() {
            if col_index < cols_offset {
                continue;
            }
            if remaining_width < hlen {
                has_more_cols_to_show = true;
                break;
            }
            let mut style = Style::default();
            if is_header {
                style = style.add_modifier(Modifier::BOLD);

            }
            let span = Span::styled((*hname).as_str(), style);
            buf.set_span(x_offset_header, y, &span, hlen);
            x_offset_header += hlen;
            remaining_width = remaining_width.saturating_sub(hlen);
        }
        state.set_more_cols_to_show(has_more_cols_to_show);
    }

    fn render_status(&self, area: Rect, buf: &mut Buffer, state: &mut CsvTableState) {
        let block = Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::Rgb(64, 64, 64)));
        block.render(area, buf);
        let style = Style::default().fg(Color::Rgb(64, 64, 64));
        let span = Span::styled(state.filename.as_str(), style);
        buf.set_span(area.x, area.bottom().saturating_sub(1), &span, area.width);
    }
}

impl<'a> StatefulWidget for CsvTable<'a> {
    type State = CsvTableState;

    fn render(mut self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {

        // TODO: draw relative to the provided area

        if area.area() == 0 {
            return;
        }

        let status_height = 2;
        let column_widths = self.get_column_widths();
        let (y_header, y_first_record) = self.render_header_borders(buf, area);

        // row area: including row numbers and row content
        let rows_area = Rect::new(
            area.x,
            y_first_record,
            area.width,
            area.height.saturating_sub(y_first_record).saturating_sub(status_height),
        );

        let row_num_section_width = self.render_row_numbers(
            buf,
            state,
            rows_area,
            self.rows.len(),
        );

        self.render_row(
            buf,
            state,
            &column_widths,
            rows_area,
            row_num_section_width,
            y_header,
            true,
            &self.header,
        );

        let mut y_offset = y_first_record;
        for row in self.rows.iter() {
            self.render_row(
                buf,
                state,
                &column_widths,
                rows_area,
                row_num_section_width,
                y_offset,
                false,
                row,
            );
            y_offset += 1;
            if y_offset >= rows_area.bottom() {
                break;
            }
        }

        let status_area = Rect::new(
            area.x,
            area.bottom().saturating_sub(status_height),
            area.width,
            status_height,
        );
        self.render_status(status_area, buf, state);
    }
}
pub struct CsvTableState {
    // TODO: types appropriate?
    rows_offset: u64,
    cols_offset: u64,
    more_cols_to_show: bool,
    filename: String,
}

impl CsvTableState {

    fn new(filename: String) -> Self {
        Self {
            rows_offset: 0,
            cols_offset: 0,
            more_cols_to_show: true,
            filename,
        }
    }

    fn set_rows_offset(&mut self, offset: u64) {
        self.rows_offset = offset;
    }

    fn set_cols_offset(&mut self, offset: u64) {
        self.cols_offset = offset;
    }

    fn set_more_cols_to_show(&mut self, value: bool) {
        self.more_cols_to_show = value;
    }

    fn has_more_cols_to_show(&mut self) -> bool {
        self.more_cols_to_show
    }

}

fn main() {
    let args: Vec<String> = env::args().collect();
    let filename = args.get(1).expect("Filename not provided");
    println!("filename: {}", filename);

    let mut num_rows = 50;
    let mut rows_from = 0;
    let mut csvlens_reader = csv::CsvLensReader::new(filename);
    let mut rows = csvlens_reader.get_rows(rows_from, num_rows).unwrap();
    let headers = csvlens_reader.headers.clone();

    let stdout = io::stdout().into_raw_mode().unwrap();
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    let events = Events::new();
    let mut csv_table_state = CsvTableState::new(filename.to_string());

    loop {
        terminal.draw(|f| {

            let size = f.size();

            // TODO: check type of num_rows too big?
            if num_rows < size.height as u64 {
                num_rows = size.height as u64;
                rows = csvlens_reader.get_rows(rows_from, num_rows).unwrap();
            }

            let csv_table = CsvTable::new(&headers, &rows);

            f.render_stateful_widget(csv_table, size, &mut csv_table_state);

        }).unwrap();

        if let Event::Input(key) = events.next().unwrap() {
            match key {
                Key::Char('q') => {
                    break;
                }
                Key::Char('j') => {
                    rows_from = rows_from + 1;
                    rows = csvlens_reader.get_rows(rows_from, num_rows).unwrap();
                }
                Key::Char('k') => {
                    if rows_from > 0 {
                        rows_from = rows_from - 1;
                        rows = csvlens_reader.get_rows(rows_from, num_rows).unwrap();
                    }
                }
                Key::Char('l') => {
                    if csv_table_state.has_more_cols_to_show() {
                        let new_cols_offset = csv_table_state.cols_offset.saturating_add(1);
                        csv_table_state.set_cols_offset(new_cols_offset);
                    }
                }
                Key::Char('h') => {
                    let new_cols_offset = csv_table_state.cols_offset.saturating_sub(1);
                    csv_table_state.set_cols_offset(new_cols_offset);
                }
                _ => {}
            }
            csv_table_state.set_rows_offset(rows_from);
        };
    }
}