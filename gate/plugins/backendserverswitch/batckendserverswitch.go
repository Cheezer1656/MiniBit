package backendserverswitch

import (
	"bytes"
	"context"
	"strconv"

	"github.com/go-logr/logr"
	"github.com/robinbraemer/event"
	"go.minekube.com/common/minecraft/component"
	"go.minekube.com/common/minecraft/key"
	"go.minekube.com/gate/pkg/edition/java/proxy"
	"go.minekube.com/gate/pkg/edition/java/proxy/message"
)

var Plugin = proxy.Plugin{
	Name: "Backend Server Switch",
	Init: func(ctx context.Context, p *proxy.Proxy) error {
		log := logr.FromContextOrDiscard(ctx)
		log.Info("Hello from Backend Server Switch!")

		key, err := key.Make("minibit", "main")
		if err != nil {
			return err
		}

		p.ChannelRegistrar().Register(&message.MinecraftChannelIdentifier{Key: key})

		pl := &plugin{proxy: p}
		event.Subscribe(p.Event(), 0, pl.onPluginMessage)

		return nil
	},
}

type plugin struct {
	proxy *proxy.Proxy
}

func (p *plugin) onPluginMessage(e *proxy.PluginMessageEvent) {
	if _, ok := e.Source().(proxy.ServerConnection); ok {
		data := bytes.Split(e.Data(), []byte("\x00"))

		msgType, err := strconv.Atoi(string(data[0]))
		if err != nil {
			return
		}

		switch msgType {
		case 1:
			username := string(data[1])
			player := p.proxy.PlayerByName(username)
			if player != nil {
				connection := player.CurrentServer()
				newServerName := string(data[2])
				newServer := p.proxy.Server(newServerName)

				if connection.Server().ServerInfo().Name() == newServerName {
					if err := player.SendMessage(&component.Text{
						Content: "You're already in that server!",
					}); err != nil {
						return
					}
				} else if newServer != nil {
					player.CreateConnectionRequest(newServer).ConnectWithIndication(context.Background())
				} else {
					if err := player.SendMessage(&component.Text{
						Content: "That server was not found!",
					}); err != nil {
						return
					}
				}
			}
		}
	}
}
