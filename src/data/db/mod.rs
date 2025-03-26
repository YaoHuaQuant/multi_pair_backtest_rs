use clickhouse::Client;

pub mod dao;
pub mod api;

pub type RDBResult<T> = Result<T, Box<dyn std::error::Error>>;

pub struct SDbClickhouse {
    pub client: Client,
}

impl SDbClickhouse {
    pub fn new() -> Self {
        // todo 参数化设置
        let client = Client::default()
            .with_url("http://192.168.3.64:8123") // 修改为你的 ClickHouse 地址
            .with_user("gx")
            .with_password("1234566")
            .with_database("btc_quant"); // 选择数据库
        Self {
            client
        }
    }

    pub fn get_client(&self) -> &Client {
        &self.client
    }
}