// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

package main

import (
	"context"
	"errors"
	"fmt"
	"net"
	"net/http"
	"sync"
	"time"
)

type RequestId = uint64

type ActiveRequests struct {
	sync.Mutex
	inner map[RequestId]*ActiveRequest
}

func (ar *ActiveRequests) exists(id RequestId) bool {
	Debug("checking if request %d exists", id)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.inner[id]
	return exists
}

func (ar *ActiveRequests) insert(id RequestId, inj ConnectionInjector) {
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.inner[id]
	if exists {
		panic("attempted to overwrite active connection")
	}
	ar.inner[id] = &ActiveRequest{injector: inj}
}

func (ar *ActiveRequests) remove(id RequestId) {
	Debug("removing request %d", id)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.inner[id]
	if !exists {
		panic("attempted to remove active connection that doesn't exist")
	}
	delete(ar.inner, id)
}

func (ar *ActiveRequests) injectData(id RequestId, data []byte) {
	Debug("injecting data for %d", id)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.inner[id]
	if !exists {
		panic("attempted to write to connection that doesn't exist")
	}
	ar.inner[id].injector.serverData <- data
}

func (ar *ActiveRequests) closeRemoteSocket(id RequestId) {
	Debug("closing remote socket for %d", id)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.inner[id]
	if !exists {
		Warn("attempted to close remote socket of a connection that doesn't exist")
		return
	}
	ar.inner[id].injector.remoteClosed <- true
}

func (ar *ActiveRequests) sendError(id RequestId, err error) {
	Debug("injecting error for %d: %s", id, err)
	ar.Lock()
	defer ar.Unlock()
	_, exists := ar.inner[id]
	if !exists {
		panic("attempted to inject error data to connection that doesn't exist")
	}
	ar.inner[id].injector.remoteError <- err
}

type ActiveRequest struct {
	injector ConnectionInjector
}

func inRedirectionLoop(req *http.Request, via []*http.Request) bool {
	target := req.URL.String()

	for i := 0; i < len(via); i++ {
		if target == via[i].URL.String() {
			return true
		}
	}
	return false
}

func checkRedirect(redirect string, req *http.Request, via []*http.Request) error {
	Debug("attempting to perform redirection to %s with our policy set to '%s'", req.URL.String(), redirect)

	if len(via) > maxRedirections {
		return errors.New(fmt.Sprintf("Maximum (%d) redirects followed", maxRedirections))
	}

	if inRedirectionLoop(req, via) {
		return errors.New("stuck in redirection loop")
	}

	redirectionChain := ""
	for i := 0; i < len(via); i++ {
		redirectionChain += fmt.Sprintf("%s -> ", via[i].URL.String())
	}
	redirectionChain += fmt.Sprintf("[%s]", req.URL.String())
	Debug("redirection chain: %s", redirectionChain)

	switch redirect {
	case REQUEST_REDIRECT_MANUAL:
		Error("unimplemented '%s' redirect", redirect)
		return http.ErrUseLastResponse
	case REQUEST_REDIRECT_ERROR:
		return errors.New("encountered redirect")
	case REQUEST_REDIRECT_FOLLOW:
		Debug("will perform redirection")
		return nil
	}

	// if this was rust that had proper enums and match statements,
	// we could have guaranteed that at compile time...
	panic("unreachable")
}

func dialContext(_ctx context.Context, _network, addr string) (net.Conn, error) {
	Info("dialing plain connection to %s", addr)

	requestId, err := rsStartNewMixnetRequest(addr)
	if err != nil {
		return nil, err
	}

	conn, inj := NewFakeConnection(requestId, addr)
	activeRequests.insert(requestId, inj)

	return conn, nil
}

func dialTLSContext(_ctx context.Context, _network, addr string) (net.Conn, error) {
	Info("dialing TLS connection to %s", addr)

	requestId, err := rsStartNewMixnetRequest(addr)
	if err != nil {
		return nil, err
	}

	conn, inj := NewFakeTlsConn(requestId, addr)
	activeRequests.insert(requestId, inj)

	if err := conn.Handshake(); err != nil {
		return nil, err
	}

	return conn, nil
}

func buildHttpClient(redirect Redirect) *http.Client {
	return &http.Client{
		CheckRedirect: func(req *http.Request, via []*http.Request) error {
			return checkRedirect(redirect, req, via)
		},

		Transport: &http.Transport{
			DialContext: func(ctx context.Context, network, addr string) (net.Conn, error) {
				return dialContext(ctx, network, addr)
			},
			DialTLSContext: func(ctx context.Context, network, addr string) (net.Conn, error) {
				return dialTLSContext(ctx, network, addr)
			},

			//TLSClientConfig: &tlsConfig,
			DisableKeepAlives:   true,
			MaxIdleConns:        1,
			MaxIdleConnsPerHost: 1,
			MaxConnsPerHost:     1,
		},
	}
}

func _closeRemoteSocket(requestId RequestId) any {
	activeRequests.closeRemoteSocket(requestId)
	return nil
}

func _injectServerData(requestId RequestId, data []byte) any {
	activeRequests.injectData(requestId, data)
	return nil
}

func _injectConnError(requestId RequestId, err error) any {
	activeRequests.sendError(requestId, err)
	return nil
}

func performRequest(req *ParsedRequest) (*http.Response, error) {
	reqClient := buildHttpClient(req.redirect)

	Info("Starting the request...")
	Debug("%s: %v", req.redirect, *req.request)
	return reqClient.Do(req.request)
}

func _mixFetch(request *ParsedRequest) (any, error) {
	Info("_mixFetch: start")

	resCh := make(chan *http.Response)
	errCh := make(chan error)
	go func(resCh chan *http.Response, errCh chan error) {
		resp, err := performRequest(request)
		if err != nil {
			errCh <- err
		} else {
			resCh <- resp
		}
	}(resCh, errCh)

	select {
	case res := <-resCh:
		Info("finished performing the request")
		Debug("response: %v", *res)
		return intoJSResponse(res)
	case err := <-errCh:
		Warn("request failure: %v", err)
		return nil, err
	case <-time.After(requestTimeout):
		// TODO: cancel stuff here.... somehow...
		Warn("request has timed out")
		return nil, errors.New("request timeout")
	}
}
