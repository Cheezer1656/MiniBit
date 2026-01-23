package minibit.minibitvelocity.commands;

import com.velocitypowered.api.command.CommandSource;
import com.velocitypowered.api.command.SimpleCommand;
import com.velocitypowered.api.proxy.Player;
import com.velocitypowered.api.proxy.ProxyServer;
import com.velocitypowered.api.proxy.ServerConnection;
import com.velocitypowered.api.proxy.server.RegisteredServer;
import net.kyori.adventure.text.Component;
import net.kyori.adventure.text.format.NamedTextColor;

import java.util.Optional;

public class LobbyCommand implements SimpleCommand {
    private RegisteredServer server;

    public LobbyCommand(ProxyServer server) {
        this.server = server.getServer("lobby").get();
    }

    @Override
    public void execute(final SimpleCommand.Invocation invocation) {
        CommandSource source = invocation.source();
        if (!(source instanceof Player)) {
            source.sendMessage(Component.text("This is a player only command!", NamedTextColor.RED));
            return;
        }

        Player player = (Player) source;
        Optional<ServerConnection> connection = player.getCurrentServer();
        if (connection.isPresent() && connection.get().getServer() == server) {
            source.sendMessage(Component.text("You're already in that server!", NamedTextColor.RED));
            return;
        }

        player.createConnectionRequest(server).fireAndForget();
    }

    @Override
    public boolean hasPermission(final Invocation invocation) {
        return true; //invocation.source().hasPermission("minibit.server.lobby");
    }
}