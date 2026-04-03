use clap::ValueEnum;

#[derive(Clone, ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
    Yaml,
    Csv,
    #[value(name = "csv-headless")]
    CsvHeadless,
}
