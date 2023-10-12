namespace DanmakuHub;

using Dapper;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Data.Sqlite;
using System.Net.Http.Headers;
using System.Security.Cryptography;

public record HashRow {
  public int Id { get; init; }
  public string Link { get; init; } = string.Empty;
  public string Hash { get; init; } = string.Empty;
}

public interface IDatabaseUtils {
  void Setup();
  ValueTask InsertAsync(HashRow row);
  ValueTask<string?> QueryAsync(string link);
}

public class DatabaseUtils : IDatabaseUtils {
  private readonly string dbConnection;

  public DatabaseUtils(string databaseName) {
    this.dbConnection = @$"Data Source={databaseName};";
  }

  public async ValueTask InsertAsync(HashRow row) {
    using var connection = new SqliteConnection(dbConnection);

    await connection.ExecuteAsync(
      @"insert into Hashes (Link, Hash)
        values (@Link, @Hash)
        on conflict (Link) do
        update set Hash = @Hash;",
      row);
  }

  public async ValueTask<string?> QueryAsync(string link) {
    using var connection = new SqliteConnection(dbConnection);

    var row = await connection.QueryFirstOrDefaultAsync<HashRow>(
      "select Link, Hash from Hashes where Link=@Link limit 1",
      new {
        Link = link,
      });

    return row?.Hash;
  }

  public void Setup() {
    using var connection = new SqliteConnection(this.dbConnection);

    var table = connection.Query<string>(@"select name from sqlite_master where type='table' and name = 'Hashes';");
    var tableName = table.FirstOrDefault();
    if (!string.IsNullOrEmpty(tableName) && tableName == "Hashes") {
      return;
    }

    connection.Execute(@"create table Hashes(
      Id INTEGER PRIMARY KEY AUTOINCREMENT,
      Link VARCHAR(1024) NOT NULL UNIQUE,
      Hash VARCHAR(1024) NOT NULL)");
  }
}

public interface IUtils {
  public ValueTask<string> Fetch16MAndCaculateMd5Async(string link);
}

public class Utils : IUtils {
  private readonly HttpClient httpClient;
  public Utils(HttpClient httpClient) {
    this.httpClient = httpClient;
  }

  public async ValueTask<string> Fetch16MAndCaculateMd5Async(string link) {
    var stream = await this.Fetch16MAsync(link);

    var md5 = MD5.Create();
    var bytes = await md5.ComputeHashAsync(stream);
    return Convert.ToHexString(bytes);
  }

  public async ValueTask<Stream> Fetch16MAsync(string link) {
    var httpRequsetMessage = new HttpRequestMessage(
        HttpMethod.Get,
        link);
    httpRequsetMessage.Headers.Range = new RangeHeaderValue(0, 16 * 1024 * 1024 - 1);

    var response = await this.httpClient.SendAsync(httpRequsetMessage);
    response.EnsureSuccessStatusCode();

    return await response.Content.ReadAsStreamAsync();
  }
}

public class Program {
  public static void Main(string[] args) {
    var builder = WebApplication.CreateBuilder(args);

    builder.Services.AddControllers();
    builder.Services.AddHttpClient();
    builder.Services.AddLogging();
    builder.Services.AddSingleton<IDatabaseUtils, DatabaseUtils>(_ => {
      var dbpath = Environment.GetEnvironmentVariable("DANMAKUHUB_DB_PATH") ?? "danmakuhub.db";
      return new DatabaseUtils(dbpath);
    });
    builder.Services.AddSingleton<IUtils, Utils>();
    builder.Services.AddCors(builder => {
      builder.AddDefaultPolicy(policy => {
        policy
            .WithOrigins("https://a.0v0.io", "https://alist.0v0.io", "https://alist.fallenwood.net", "http://localhost:5173")
            .WithMethods("POST");
      });

    });

    var app = builder.Build();

    var allowedLinks = Environment.GetEnvironmentVariable("DANMAKUHUB_ALLOWED_LINKS")?.Split(';') ?? Array.Empty<string>();

    app.Services.GetRequiredService<IDatabaseUtils>().Setup();

    app.UseCors();

    app.UseAuthorization();

    app.MapPost(
      "/get_md5",
      async (
        [FromQuery] string? link,
        [FromServices] IUtils utils,
        [FromServices] IDatabaseUtils databaseUtils,
        [FromServices] ILoggerFactory loggerFactory) => {
          if (string.IsNullOrWhiteSpace(link)) {
            return Results.BadRequest();
          }

          if (!allowedLinks.Any(e => link.StartsWith(e, StringComparison.InvariantCultureIgnoreCase))) {
            return Results.BadRequest();
          }

          var existing = await databaseUtils.QueryAsync(link);

          if (existing != null) {
            return Results.Ok(new {
              Hash = existing
            });
          }

          var logger = loggerFactory.CreateLogger("Md5-Background");
          _ = Task.Run(async () => {
            try {
              var hex = await utils.Fetch16MAndCaculateMd5Async(link);
              await databaseUtils.InsertAsync(new HashRow {
                Hash = hex,
                Link = link,
              });
            } catch (HttpRequestException hre) {
              logger.Log(LogLevel.Error, "Failed to get md5 for {0}, {1}", link, hre);
            } catch (Exception e) {
              logger.Log(LogLevel.Error, "Exception for {0}, {1}", link, e);
            }
          });

          return Results.NotFound();
        });

    app.Run();
  }
}
