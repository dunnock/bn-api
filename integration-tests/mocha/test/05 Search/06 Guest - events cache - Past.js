const supertest = require('supertest');
const expect = require('chai').expect;
const mocha = require('mocha');
const tv4 = require('tv4');
const fs = require('fs');
const pm = require('../pm');const debug = require("debug");var log=debug('bn-api');

const baseUrl = supertest(pm.environment.get('server'));

const apiEndPoint = '/events?query=&page=0&status=Published';


var response;
var responseBody;
var cached_response;
var cachedResponseBody;


const post = async function (request_body) {
    return baseUrl
        .post(pm.substitute(apiEndPoint))
        .set('Accept', 'application/json')
        .set('Content-Type', 'application/json')

        .send(pm.substitute(request_body));
};

const get = async function (request_body) {
    return baseUrl
        .get(pm.substitute(apiEndPoint))

        .set('Accept', 'application/json')
        .send();
};

const get_cached = async function (request_body, etag) {
    return baseUrl
        .get(pm.substitute(apiEndPoint))

        .set('If-None-Match', etag)
        .set('Accept', 'application/json')
        .send();
};

let requestBody = ``;


describe('Guest - events cache - Published', function () {
    before(async function () {
        response = await get(requestBody);
        log(response.request.header);
        log(response.request.url);
        log(response.request._data);
        log(response.request.method);
        responseBody = JSON.stringify(response.body);
        //log(pm);
        log(response.status);
        log(responseBody);

        etag = response.header['etag'];
        cached_response = await get_cached(requestBody, etag);
        cachedResponseBody = cached_response.body;
        log(cached_response.status);
        log(cachedResponseBody);
        cachedResponseBody
    });

    after(async function () {
        // add after methods


    });

    it("first response should be 200", function () {
        expect(response.status).to.equal(200);
    })

    it("second response should be 304", function () {
        expect(cached_response.status).to.equal(304);
    })

    it("second response should be empty", function () {
        expect(cachedResponseBody).to.equal("");
    })

});

            
