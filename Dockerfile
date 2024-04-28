FROM rust:1.76

RUN apt install libssl-dev

WORKDIR /app

COPY . .

RUN mkdir "backup"

RUN cd /app
RUN pwd && ls -ltrh

RUN cargo install --path .

CMD ["github"]
