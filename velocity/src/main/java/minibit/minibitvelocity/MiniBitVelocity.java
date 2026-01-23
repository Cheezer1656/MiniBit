
package minibit.minibitvelocity;

import com.google.common.io.ByteArrayDataInput;
import com.google.common.io.ByteStreams;
import com.google.inject.Inject;
import com.velocitypowered.api.command.CommandManager;
import com.velocitypowered.api.command.CommandMeta;
import com.velocitypowered.api.event.connection.PluginMessageEvent;
import com.velocitypowered.api.event.proxy.ProxyInitializeEvent;
import com.velocitypowered.api.event.Subscribe;
import com.velocitypowered.api.plugin.Plugin;
import com.velocitypowered.api.proxy.Player;
import com.velocitypowered.api.proxy.ProxyServer;
import com.velocitypowered.api.proxy.ServerConnection;
import com.velocitypowered.api.proxy.messages.MinecraftChannelIdentifier;
import com.velocitypowered.api.proxy.server.RegisteredServer;
import minibit.minibitvelocity.commands.LobbyCommand;
import net.kyori.adventure.text.Component;
import org.slf4j.Logger;

import java.util.Optional;

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

        CommandManager commandManager = server.getCommandManager();
        CommandMeta commandMeta = commandManager.metaBuilder("lobby")
                .aliases("l")
                .plugin(this)
                .build();

        commandManager.register(commandMeta, new LobbyCommand(server));
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
        String[] data = in.readLine().split("\0");
        int type = Integer.valueOf(data[0]);
        if (type == 1) {
            Optional<Player> player_op = server.getPlayer(data[1]);
            if (player_op.isPresent() && player_op.get().getCurrentServer().isPresent()) {
                Player player = player_op.get();
                ServerConnection connection = player.getCurrentServer().get();
                Optional<RegisteredServer> server2 = server.getServer(data[2]);
                if (connection.getServer().toString() == data[2]) {
                    player.sendMessage(Component.text("You're already in that server!"));
                } else if (server2.isPresent()) {
                    player.createConnectionRequest(server2.get()).fireAndForget();
                } else {
                    player.sendMessage(Component.text("That server was not found!"));
                }
            }
        }
    }
}
