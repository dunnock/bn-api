const supertest = require('supertest');
const expect = require('chai').expect;
const mocha = require('mocha');
const tv4 = require('tv4');
const fs = require('fs');
const pm = require('../pm');
const debug = require('debug');
var log = debug('bn-api');
const events = require('../../helpers/events');
const cart = require('../../helpers/cart');
const querystring = require("querystring");

const baseUrl = supertest("https://flex-api.sharetribe.com/v1/");

const authEndpoint = "auth/token";
const apiEndPoint = 'api/listings/query';


var response;
var responseBody;


const postAuth = async function () {
    return baseUrl
        .post(authEndpoint).set("Content-Type", "application/x-www-form-urlencoded").send(
            {
                "client_id": pm.substitute("{{flex_client_id}}"),
                "grant_type": "client_credentials",
                "scope": "public-read"
            });
};

const get = async function (request_body) {
    return baseUrl
        .get(pm.substitute(apiEndPoint))
        .set('Authorization', pm.substitute('Bearer {{flex_anon_token}}'))
        .set('Accept', 'application/json')
        .send();
};

let requestBody = ``;


describe('User - Search listings', function () {
    before(async function () {
        this.timeout(100000);
        let auth = await postAuth();
        console.log(auth.body);
        expect(auth.status).to.equal(200);

         auth = auth.body;
        pm.environment.set("flex_anon_token", auth.access_token);
        response = await get(requestBody);
        log(response.request.header);
        log(response.request.url);
        log(response.request._data);
        log(response.request.method);
        responseBody = JSON.stringify(response.body);
        //log(pm);
        log(response.status);
        log(responseBody);
    });


    it("should be 200", function () {
        expect(response.status).to.equal(200);
        let json = JSON.parse(responseBody);
        console.log(responseBody);
        pm.environment.set("flex_listing_id", json.data[0].id);
        expect(json.data.len > 0);
    })


});

            
