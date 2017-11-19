FROM scratch
ADD rust-mongodb-server /
CMD ["/rust-mongodb-server"]
