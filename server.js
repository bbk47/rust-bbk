const net = require("net");

const server = net.createServer((socket) => {
    let buf=Buffer.from([]);
    console.log('connecting....',socket.address());
  socket.on("data", (data) => {
    console.log('len:',data.length)
    buf = Buffer.concat([buf,data]);
    console.log(data.toString());
    console.log('write=====')
    socket.write("HTTP/1.1 200 OK\n\nhallo world");
    console.log('end====')
    socket.end();
  });
});

server.listen(3000);