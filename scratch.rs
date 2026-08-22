use tokio_postgres::{NoTls, Error};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let (client, mut connection) = tokio_postgres::connect("host=localhost user=postgres", NoTls).await?;
    
    Ok(())
}
