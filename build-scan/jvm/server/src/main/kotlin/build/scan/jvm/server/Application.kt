package build.scan.jvm.server

import io.ktor.server.application.Application
import io.ktor.server.engine.embeddedServer
import io.ktor.server.netty.Netty
import io.ktor.server.response.respondText
import io.ktor.server.routing.get
import io.ktor.server.routing.routing

fun main() {
  val port = System.getenv("PORT")?.toIntOrNull() ?: 8080
  val host = System.getenv("HOST") ?: "0.0.0.0"
  embeddedServer(Netty, port = port, host = host, module = Application::module)
      .start(wait = true)
}
}

fun Application.module() {
  routing { get("/health") { call.respondText("OK") } }
}
