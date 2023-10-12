FROM mcr.microsoft.com/dotnet/aspnet:7.0-cbl-mariner2.0-distroless-amd64 AS base
WORKDIR /app
EXPOSE 80

FROM mcr.microsoft.com/dotnet/sdk:7.0 AS build
WORKDIR "/src"
COPY . .
RUN dotnet restore "DanmakuHub/DanmakuHub.csproj"
WORKDIR "/src/DanmakuHub"
RUN dotnet build "DanmakuHub.csproj" -c Release -o /app/build

FROM build AS publish
RUN dotnet publish "DanmakuHub.csproj" -c Release -o /app/publish /p:UseAppHost=false

FROM base AS final
WORKDIR /app
COPY --from=publish /app/publish .

ENV ASPNETCORE_URLS=http://+:80
ENV ASPNETCORE_ENVIRONMENT=Production

ENV DOTNET_TieredPGO=1
ENV DOTNET_ReadyToRun=0
ENV DOTNET_TC_QuickJitForLoops=1

ENTRYPOINT ["dotnet", "DanmakuHub.dll"]
