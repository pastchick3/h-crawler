use std::path::PathBuf;

const USER_AGENT: &str = concat!(
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ",
    "AppleWebKit/537.36 (KHTML, like Gecko) ",
    "Chrome/81.0.4044.138 ",
    "Safari/537.36 Edg/81.0.416.72",
);

struct Progress {
    title: String,
    done: RefCell<usize>,
    total: usize,
}

impl Display for Progress {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{} => {}/{}", self.title, self.done.borrow(), self.total)
    }
}

impl Progress {
    fn new(title: &str, total: usize) -> Self {
        Progress {
            title: title.to_string(),
            done: RefCell::new(0),
            total,
        }
    }

    fn make_progress(&self) {
        *self.done.borrow_mut() += 1;
    }

    fn print_progress(&self) {
        print!("\r{}", self);
        io::stdout().flush().unwrap();
    }
}

pub struct Crawler {

}

impl Crawler {
    pub fn new(output: PathBuf,timeout: usize, retry:usize, concurrency: usize) -> Self {
        Self {}
    }
}
