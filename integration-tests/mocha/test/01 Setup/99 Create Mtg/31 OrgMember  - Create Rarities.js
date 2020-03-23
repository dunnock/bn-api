const supertest = require('supertest');
const expect = require('chai').expect;
const mocha = require('mocha');
const tv4 = require('tv4');
const fs = require('fs');
const pm = require('../../pm');const debug=require('debug');var log = debug('bn-api');

const baseUrl = supertest(pm.environment.get('server'));

const apiEndPoint = '/events/{{last_event_id}}/rarities';


var response;
var responseBody;


const post = async function (request_body) {
    return baseUrl
        .post(pm.substitute(apiEndPoint))
        .set('Accept', 'application/json')
        .set('Content-Type', 'application/json')
        .set('Authorization', pm.substitute('Bearer {{org_member_token}}'))

        .send(pm.substitute(request_body));
};

const get = async function (request_body) {
    return baseUrl
        .get(pm.substitute(apiEndPoint))

        .set('Authorization', pm.substitute('Bearer {{org_member_token}}'))

        .set('Accept', 'application/json')
        .send();
};

let requestBodies = [
    { data: `{"name":"Mythic", "rank":4}`, varName: "mythic"},
    { data: `{"name":"Rare", "rank":3}`, varName: "rare"},
    { data: `{"name":"Uncommon", "rank":2}`, varName: "uncommon"},
    { data: `{"name":"Common", "rank":1}`, varName: "common"}];

var responses = [];
describe('OrgMember  - Create Rarities', function () {
    before(async function () {
        responses =await requestBodies.map(async r =>{
            response = await post(r.data);
            log(response.request.header);
            log(response.request.url);
            log(response.request._data);
            log(response.request.method);
           // console.log(response);
           // expect(response.status).to.equal(201);
            responseBody = JSON.stringify(response.body);
            pm.environment.set(r.varName, JSON.parse(responseBody).id);
            //log(pm);
            log(response.status);
            log(responseBody);
            return {status:response.status, body: responseBody};
        });
        responses = await Promise.all(responses);

    });

    after(async function () {
        // add after methods



    });

    it("should be 201", function () {
        console.log(responses);
        for (var i=0;i<requestBodies.length;i++){
            expect(responses[i].status).to.equal(201);
        }

    })


});


