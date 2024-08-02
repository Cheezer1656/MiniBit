/*
    MiniBit - A Minecraft minigame server network written in Rust.
    Copyright (C) 2024  Cheezer1656 (https://github.com/Cheezer1656/)

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as published
    by the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

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