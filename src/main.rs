use std::collections::BTreeSet;
use std::env;
use std::error::Error;
use std::io;

use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use tfrecord::protos::{event::What, summary::value::Value};
use tfrecord::reader::EventReader;
use tfrecord::RecordReaderInit;
use tui::backend::CrosstermBackend;
use tui::symbols::Marker;
use tui::widgets::{Axis, Chart, Dataset, GraphType};
use tui::Terminal;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        println!("usage: {} EVENTS_FILE [SCALAR_TAG]", args[0]);
        return Ok(());
    }

    let reader: EventReader<_> = RecordReaderInit::default().open(&args[1])?;
    if args.len() == 2 {
        println!("Available scalars:");
        for tag in get_tags(reader)? {
            println!("  {}", tag);
        }
        return Ok(());
    }

    let mut data = Vec::new();
    for event in reader {
        let event = event?;
        if let Some(What::Summary(summary)) = event.what {
            for value in summary.value {
                if args[2] == value.tag {
                    if let Some(Value::SimpleValue(v)) = value.value {
                        data.push((event.step as f64, v as f64));
                    }
                }
            }
        }
    }

    if data.is_empty() {
        println!("No events found for {}", args[2]);
    } else {
        draw_chart(&args[2], &data)?;
    }

    Ok(())
}

fn get_tags<T: io::Read>(reader: EventReader<T>) -> Result<Vec<String>, Box<dyn Error>> {
    let mut tags = BTreeSet::new();
    for event in reader {
        if let Some(What::Summary(summary)) = event?.what {
            for value in summary.value {
                if let Some(Value::SimpleValue(_)) = value.value {
                    tags.insert(value.tag);
                }
            }
        }
    }
    Ok(tags.into_iter().collect())
}

fn draw_chart(name: &str, data: &[(f64, f64)]) -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;

    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    execute!(terminal.backend_mut(), EnterAlternateScreen)?;

    let bounds = data_bounds(data);
    let dataset = Dataset::default()
        .name(name)
        .data(data)
        .marker(Marker::Braille)
        .graph_type(GraphType::Line);
    let chart = Chart::new(vec![dataset])
        .x_axis(Axis::default().bounds(bounds.0))
        .y_axis(Axis::default().bounds(bounds.1));

    terminal.draw(|f| {
        f.render_widget(chart, f.size());
    })?;

    loop {
        use crossterm::event::{self, Event};
        if let Event::Key(_) = event::read()? {
            break;
        }
    }

    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

fn data_bounds(data: &[(f64, f64)]) -> ([f64; 2], [f64; 2]) {
    let (x, y) = data[0];
    let acc = ([x, x], [y, y]);
    data.iter().fold(acc, |acc, d| {
        (
            [f64::min(acc.0[0], d.0), f64::max(acc.0[1], d.0)],
            [f64::min(acc.1[0], d.1), f64::max(acc.1[1], d.1)],
        )
    })
}
