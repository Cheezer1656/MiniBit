package lobbycommand

import (
	"context"

	"github.com/go-logr/logr"
	"go.minekube.com/brigodier"
	"go.minekube.com/common/minecraft/color"
	"go.minekube.com/common/minecraft/component"
	"go.minekube.com/gate/pkg/command"
	"go.minekube.com/gate/pkg/edition/java/proxy"
)

var Plugin = proxy.Plugin{
	Name: "Lobby Command",
	Init: func(ctx context.Context, p *proxy.Proxy) error {
		log := logr.FromContextOrDiscard(ctx)
		log.Info("Hello from Lobby Command!")

		lobby := "lobby"

		p.Command().RegisterWithAliases(
			brigodier.Literal("lobby").Executes(command.Command(func(c *command.Context) error {
				player, ok := c.Source.(proxy.Player)
				if !ok {
					return c.Source.SendMessage(&component.Text{
						Content: "This is a player only command!",
						S:       component.Style{Color: color.Red}})
				}

				if player.CurrentServer().Server().ServerInfo().Name() == lobby {
					return c.Source.SendMessage(&component.Text{
						Content: "You're already in that server!",
						S:       component.Style{Color: color.Red}})
				}

				player.CreateConnectionRequest(p.Server(lobby)).ConnectWithIndication(context.Background())

				return nil
			})),
			"l",
		)

		return nil
	},
}
