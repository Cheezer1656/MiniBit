package minibit.minibitvelocity;

import com.google.common.io.ByteArrayDataInput;
import com.google.common.io.ByteStreams;
import com.google.inject.Inject;
import com.velocitypowered.api.event.connection.PluginMessageEvent;
import com.velocitypowered.api.event.proxy.ProxyInitializeEvent;
import com.velocitypowered.api.event.Subscribe;
import com.velocitypowered.api.plugin.Plugin;
import com.velocitypowered.api.proxy.ProxyServer;
import com.velocitypowered.api.proxy.ServerConnection;
import com.velocitypowered.api.proxy.messages.MinecraftChannelIdentifier;
import org.slf4j.Logger;

import java.nio.charset.StandardCharsets;

@Plugin(
        id = "minibit",
        name = "MiniBitVelocity",
        version = BuildConstants.VERSION,
        authors = {"Enderix"}
)
public class MiniBitVelocity {
    private final ProxyServer server;
    private final Logger logger;
    public static final MinecraftChannelIdentifier IDENTIFIER = MinecraftChannelIdentifier.from("minibit:main");

    @Inject
    public MiniBitVelocity(ProxyServer server, Logger logger) {
        this.server = server;
        this.logger = logger;
    }

    @Subscribe
    public void onProxyInitialization(ProxyInitializeEvent event) {
        server.getChannelRegistrar().register(IDENTIFIER);
    }

    @Subscribe
    public void onPluginMessageFromBackend(PluginMessageEvent event) {
        if (!(event.getSource() instanceof ServerConnection backend)) {
            return;
        }
        if (event.getIdentifier() != IDENTIFIER) {
            return;
        }

        ByteArrayDataInput in = ByteStreams.newDataInput(event.getData());
        String message = in.readUTF();
        logger.info("Received message from backend: " + message);
        backend.sendPluginMessage(IDENTIFIER, message.getBytes());
    }
}
