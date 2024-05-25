FROM rust:1.68 as bhcli-builder
RUN apt-get update
RUN apt-get install -y pkg-config libasound2-dev libssl-dev cmake libfreetype6-dev libexpat1-dev libxcb-composite0-dev libx11-dev
